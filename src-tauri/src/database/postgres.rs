use super::{
    Capabilities, Cell, ColumnMeta, ConnectParams, DatabaseSession, DbObject, Engine, ObjectKind,
    ObjectStructure, QueryResult, ServerInfo, TextRows, TlsMode, error, first_column, metadata,
    quote,
};
use crate::errors::AppError;
use futures_util::TryStreamExt;
use sqlx::{
    AssertSqlSafe, Column, ConnectOptions, Connection, Either, Row, TypeInfo,
    postgres::{PgConnectOptions, PgConnection, PgSslMode},
};

pub struct PgSession {
    connection: PgConnection,
    info: ServerInfo,
    /// Server process id, used to cancel a running query from another session.
    backend_pid: i32,
    host: String,
    port: u16,
}

impl PgSession {
    /// A PostgreSQL session can only read the database it connected to.
    fn require_current(&self, database: &str) -> Result<(), AppError> {
        match self.info.database.as_deref() {
            Some(current) if current != database => {
                Err(error::database_not_open(database, current))
            }
            _ => Ok(()),
        }
    }
}

impl PgSession {
    pub fn backend_pid(&self) -> i32 {
        self.backend_pid
    }
}

fn context(params: &ConnectParams) -> error::Context<'_> {
    error::Context {
        engine: Engine::Postgres,
        host: &params.host,
        port: params.port,
        database: params.database.as_deref(),
        password: params.password.as_deref(),
        tls: params.tls,
    }
}

impl DatabaseSession for PgSession {
    async fn connect(params: &ConnectParams) -> Result<Self, AppError> {
        let (target_host, target_port) = match params.connect_to {
            Some(address) => (address.ip().to_string(), address.port()),
            None => (params.host.clone(), params.port),
        };
        let mut options = PgConnectOptions::new()
            .host(&target_host)
            .port(target_port)
            .username(&params.username)
            .application_name("Thabi Studio")
            .ssl_mode(match params.tls {
                // Through a tunnel the TLS peer is 127.0.0.1, so only the
                // certificate chain can be checked; SSH host keys protect the path.
                TlsMode::Verify if params.connect_to.is_some() => PgSslMode::VerifyCa,
                TlsMode::Verify => PgSslMode::VerifyFull,
                TlsMode::EncryptOnly => PgSslMode::Require,
                TlsMode::Off => PgSslMode::Disable,
            });
        if let Some(password) = &params.password {
            options = options.password(password);
        }
        if let Some(database) = &params.database {
            options = options.database(database);
        }
        if let Some(ca) = &params.ca_path {
            options = options.ssl_root_cert(ca);
        }
        let connection = options
            .connect()
            .await
            .map_err(|e| error::from_sqlx(e, context(params)))?;

        let mut session = Self {
            connection,
            info: ServerInfo {
                engine: Engine::Postgres,
                version: String::new(),
                database: None,
            },
            backend_pid: 0,
            host: params.host.clone(),
            port: params.port,
        };
        let row = session
            .execute(
                "SELECT current_setting('server_version'), pg_backend_pid(), current_database()",
                1,
            )
            .await?;
        let mut cells = row.rows.into_iter().next().unwrap_or_default().into_iter();
        if let Some(Cell::Value(version)) = cells.next() {
            let number = version.split_whitespace().next().unwrap_or(&version);
            session.info.version = format!("PostgreSQL {number}");
        }
        if let Some(Cell::Value(pid)) = cells.next() {
            session.backend_pid = pid.parse().unwrap_or_default();
        }
        if let Some(Cell::Value(database)) = cells.next() {
            session.info.database = Some(database);
        }
        Ok(session)
    }

    fn server_info(&self) -> &ServerInfo {
        &self.info
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            cancel_query: true,
            schemas: true,
            rows_affected: true,
            multiple_result_sets: false,
        }
    }

    async fn ping(&mut self) -> Result<(), AppError> {
        let host = self.host.clone();
        let ctx = error::session_context(Engine::Postgres, &host, self.port);
        self.connection
            .ping()
            .await
            .map_err(|e| error::from_sqlx(e, ctx))
    }

    async fn list_databases(&mut self) -> Result<Vec<String>, AppError> {
        let result = self
            .execute(
                "SELECT datname FROM pg_database WHERE datallowconn AND NOT datistemplate ORDER BY datname",
                10_000,
            )
            .await?;
        Ok(first_column(result))
    }

    async fn execute(&mut self, sql: &str, max_rows: usize) -> Result<QueryResult, AppError> {
        let host = self.host.clone();
        let ctx = error::session_context(Engine::Postgres, &host, self.port);
        // The simple query protocol returns every value as text, which is
        // exactly what the results grid shows.
        let mut stream =
            sqlx::raw_sql(AssertSqlSafe(sql.to_owned())).fetch_many(&mut self.connection);
        let mut result = QueryResult::default();
        let mut row_sets = 0usize;
        let mut in_row_set = false;
        while let Some(item) = stream
            .try_next()
            .await
            .map_err(|e| error::from_sqlx(e, ctx))?
        {
            match item {
                Either::Left(done) => {
                    in_row_set = false;
                    *result.rows_affected.get_or_insert(0) += done.rows_affected();
                }
                Either::Right(row) => {
                    if !in_row_set {
                        in_row_set = true;
                        row_sets += 1;
                    }
                    if row_sets > 1 {
                        continue;
                    }
                    if result.columns.is_empty() {
                        result.columns = row
                            .columns()
                            .iter()
                            .map(|c| ColumnMeta {
                                name: c.name().to_owned(),
                                type_name: c.type_info().name().to_owned(),
                            })
                            .collect();
                    }
                    if result.rows.len() >= max_rows {
                        result.truncated = true;
                        break;
                    }
                    result.rows.push(
                        (0..row.len())
                            .map(|i| match row.try_get_unchecked::<Option<String>, _>(i) {
                                Ok(Some(text)) => Cell::Value(text),
                                Ok(None) => Cell::Null,
                                Err(_) => match row.try_get_unchecked::<Option<Vec<u8>>, _>(i) {
                                    Ok(Some(bytes)) => Cell::binary(&bytes),
                                    _ => Cell::Null,
                                },
                            })
                            .collect(),
                    );
                }
            }
        }
        result.extra_result_sets = row_sets.saturating_sub(1);
        if !result.columns.is_empty() {
            result.rows_affected = None;
        }
        Ok(result)
    }

    async fn query_text(&mut self, sql: &str, params: &[&str]) -> Result<TextRows, AppError> {
        let host = self.host.clone();
        let ctx = error::session_context(Engine::Postgres, &host, self.port);
        let mut query = sqlx::query(AssertSqlSafe(sql.to_owned()));
        for param in params {
            query = query.bind(*param);
        }
        let rows = query
            .fetch_all(&mut self.connection)
            .await
            .map_err(|e| error::from_sqlx(e, ctx))?;
        Ok(rows
            .iter()
            .map(|row| {
                (0..row.len())
                    .map(|i| row.try_get::<Option<String>, _>(i).ok().flatten())
                    .collect()
            })
            .collect())
    }

    async fn list_schemas(&mut self, database: &str) -> Result<Option<Vec<String>>, AppError> {
        self.require_current(database)?;
        let rows = self
            .query_text(
                "SELECT nspname::text FROM pg_namespace
                 WHERE nspname NOT LIKE 'pg\\_%' ESCAPE '\\' AND nspname <> 'information_schema'
                 ORDER BY nspname",
                &[],
            )
            .await?;
        Ok(Some(metadata::names(rows)))
    }

    async fn list_objects(
        &mut self,
        database: &str,
        schema: &str,
    ) -> Result<Vec<DbObject>, AppError> {
        self.require_current(database)?;
        let rows = self
            .query_text(
                "SELECT c.relname::text,
                        CASE c.relkind WHEN 'v' THEN 'view' WHEN 'm' THEN 'materialized_view'
                                       WHEN 'f' THEN 'foreign_table' ELSE 'table' END
                 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = $1 AND c.relkind IN ('r', 'p', 'v', 'm', 'f')
                 ORDER BY c.relname",
                &[schema],
            )
            .await?;
        Ok(metadata::object_rows(rows))
    }

    async fn describe(
        &mut self,
        database: &str,
        schema: &str,
        name: &str,
        kind: ObjectKind,
    ) -> Result<ObjectStructure, AppError> {
        self.require_current(database)?;
        let columns = self
            .query_text(
                "SELECT a.attname::text,
                        format_type(a.atttypid, a.atttypmod),
                        (NOT a.attnotnull)::text,
                        pg_get_expr(d.adbin, d.adrelid),
                        (SELECT k.ord::text
                         FROM pg_index i, unnest(i.indkey::int2[]) WITH ORDINALITY AS k(attnum, ord)
                         WHERE i.indrelid = c.oid AND i.indisprimary AND k.attnum = a.attnum)
                 FROM pg_attribute a
                 JOIN pg_class c ON c.oid = a.attrelid
                 JOIN pg_namespace n ON n.oid = c.relnamespace
                 LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
                 WHERE n.nspname = $1 AND c.relname = $2 AND a.attnum > 0 AND NOT a.attisdropped
                 ORDER BY a.attnum",
                &[schema, name],
            )
            .await?;
        let indexes = if matches!(kind, ObjectKind::Table | ObjectKind::MaterializedView) {
            let rows = self
                .query_text(
                    "SELECT ic.relname::text, i.indisunique::text, i.indisprimary::text,
                            (SELECT string_agg(a.attname, ', ' ORDER BY k.ord)
                             FROM unnest(i.indkey::int2[]) WITH ORDINALITY AS k(attnum, ord)
                             JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = k.attnum)
                     FROM pg_index i
                     JOIN pg_class ic ON ic.oid = i.indexrelid
                     JOIN pg_class c ON c.oid = i.indrelid
                     JOIN pg_namespace n ON n.oid = c.relnamespace
                     WHERE n.nspname = $1 AND c.relname = $2
                     ORDER BY ic.relname",
                    &[schema, name],
                )
                .await?;
            Some(metadata::index_rows(rows))
        } else {
            None
        };
        Ok(ObjectStructure {
            columns: metadata::column_rows(columns),
            indexes,
        })
    }

    fn preview_sql(&self, _database: &str, schema: &str, name: &str, limit: usize) -> String {
        format!(
            "SELECT * FROM {} LIMIT {limit}",
            quote::qualified(Engine::Postgres, &[schema, name])
        )
    }

    async fn close(self) -> Result<(), AppError> {
        self.connection
            .close()
            .await
            .map_err(|_| AppError::connection_close_failed())
    }
}

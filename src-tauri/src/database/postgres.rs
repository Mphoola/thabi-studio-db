use super::{
    Capabilities, Cell, ColumnMeta, ConnectParams, DatabaseSession, DbObject, Engine, ObjectKind,
    ObjectStructure, QueryResult, ServerInfo, TextRows, TlsMode, error, first_column, metadata,
    quote, values,
};
use crate::errors::AppError;
use futures_util::TryStreamExt;
use sqlx::postgres::PgRow;
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

/// One cell of a bound query's result, as the grid shows it.
///
/// A bound statement goes through the prepared protocol, which returns values
/// typed rather than as text, so each column is read as its own type and
/// rendered here. `NUMERIC` is the reason this cannot simply be a string:
/// reading it as a float would quietly drop digits the grid exists to show
/// exactly.
///
/// A type that is not listed is an error naming it, never a blank cell. The
/// text path degrades a failed read to NULL, which is right there because the
/// simple protocol hands back everything as text and there is nothing to
/// misread; here it would hide a value the user asked to see.
fn bound_cell(row: &PgRow, index: usize, type_name: &str) -> Result<Cell, AppError> {
    let unsupported = || {
        AppError::new(
            "unsupported_type",
            format!("A column of type {type_name} cannot be shown in a bound query."),
        )
    };
    macro_rules! as_text {
        ($type:ty, $render:expr) => {
            row.try_get::<Option<$type>, _>(index)
                .map(|value| match value {
                    Some(value) => Cell::Value(($render)(value)),
                    None => Cell::Null,
                })
                .map_err(|_| unsupported())
        };
    }
    match type_name {
        "BOOL" => as_text!(bool, |value: bool| {
            if value {
                "t".to_string()
            } else {
                "f".to_string()
            }
        }),
        "INT2" => as_text!(i16, |value: i16| value.to_string()),
        "INT4" => as_text!(i32, |value: i32| value.to_string()),
        "INT8" => as_text!(i64, |value: i64| value.to_string()),
        "FLOAT4" => as_text!(f32, |value: f32| value.to_string()),
        "FLOAT8" => as_text!(f64, |value: f64| value.to_string()),
        // Trailing zeros are dropped, because the simple protocol drops them
        // too: a bound query has to show what an unbound one shows, or the
        // same column reads differently depending on whether it had a
        // parameter. No digit that is not there is added or lost.
        "NUMERIC" => as_text!(sqlx::types::BigDecimal, |value: sqlx::types::BigDecimal| {
            let text = value.to_string();
            if !text.contains('.') {
                return text;
            }
            let trimmed = text.trim_end_matches('0');
            trimmed.trim_end_matches('.').to_string()
        }),
        "TEXT" | "VARCHAR" | "BPCHAR" | "CHAR" | "NAME" | "CITEXT" | "UNKNOWN" => row
            .try_get::<Option<String>, _>(index)
            .map(|value| match value {
                Some(text) => Cell::Value(text),
                None => Cell::Null,
            })
            .map_err(|_| unsupported()),
        "JSON" | "JSONB" => as_text!(serde_json::Value, |value: serde_json::Value| value
            .to_string()),
        "UUID" => as_text!(sqlx::types::Uuid, |value: sqlx::types::Uuid| value
            .to_string()),
        "DATE" => as_text!(chrono::NaiveDate, |value: chrono::NaiveDate| value
            .to_string()),
        "TIME" => as_text!(chrono::NaiveTime, |value: chrono::NaiveTime| value
            .to_string()),
        "TIMESTAMP" => as_text!(chrono::NaiveDateTime, |value: chrono::NaiveDateTime| value
            .to_string()),
        "TIMESTAMPTZ" => as_text!(chrono::DateTime<chrono::Utc>, |value: chrono::DateTime<
            chrono::Utc,
        >| value.to_string()),
        "BYTEA" => row
            .try_get::<Option<Vec<u8>>, _>(index)
            .map(|value| match value {
                Some(bytes) => Cell::binary(&bytes),
                None => Cell::Null,
            })
            .map_err(|_| unsupported()),
        other => Err(AppError::new(
            "unsupported_type",
            format!("A column of type {other} cannot be shown in a bound query."),
        )),
    }
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
        // Mutual TLS: present this certificate and key to the server.
        if let (Some(cert), Some(key)) = (&params.client_cert, &params.client_key) {
            options = options.ssl_client_cert(cert).ssl_client_key(key);
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

    async fn execute_bound(
        &mut self,
        sql: &str,
        params: &[values::Param],
        max_rows: usize,
    ) -> Result<QueryResult, AppError> {
        let host = self.host.clone();
        let ctx = error::session_context(Engine::Postgres, &host, self.port);
        // Bound, so the prepared protocol: the values never touch the text.
        let mut query = sqlx::query(AssertSqlSafe(sql.to_owned()));
        for param in params {
            query = match param {
                values::Param::Text(text) => query.bind(text.clone()),
                values::Param::Number(text) => {
                    // Exact: an integer as int8, anything else as NUMERIC.
                    if let Ok(integer) = text.trim().parse::<i64>() {
                        query.bind(integer)
                    } else {
                        match text.trim().parse::<sqlx::types::BigDecimal>() {
                            Ok(decimal) => query.bind(decimal),
                            Err(_) => {
                                return Err(AppError::new(
                                    "invalid_parameter",
                                    format!("“{text}” isn't a number."),
                                ));
                            }
                        }
                    }
                }
                values::Param::Null => query.bind(Option::<String>::None),
            };
        }
        let rows = query
            .fetch_all(&mut self.connection)
            .await
            .map_err(|e| error::from_sqlx(e, ctx))?;
        let mut result = QueryResult::default();
        if let Some(first) = rows.first() {
            result.columns = first
                .columns()
                .iter()
                .map(|column| ColumnMeta {
                    name: column.name().to_owned(),
                    type_name: column.type_info().name().to_owned(),
                })
                .collect();
        }
        let types: Vec<String> = result
            .columns
            .iter()
            .map(|column| column.type_name.clone())
            .collect();
        for row in rows.iter() {
            if result.rows.len() >= max_rows {
                result.truncated = true;
                break;
            }
            let mut cells = Vec::with_capacity(types.len());
            for (index, type_name) in types.iter().enumerate() {
                cells.push(bound_cell(row, index, type_name)?);
            }
            result.rows.push(cells);
        }
        if !result.columns.is_empty() {
            result.rows_affected = None;
        }
        Ok(result)
    }

    async fn execute_into(&mut self, sql: &str, sink: &values::SharedSink) -> Result<(), AppError> {
        let host = self.host.clone();
        let ctx = error::session_context(Engine::Postgres, &host, self.port);
        // The simple query protocol returns every value as text, which is
        // exactly what the results grid shows. Every row set is kept.
        let mut stream =
            sqlx::raw_sql(AssertSqlSafe(sql.to_owned())).fetch_many(&mut self.connection);
        while let Some(item) = stream
            .try_next()
            .await
            .map_err(|e| error::from_sqlx(e, ctx))?
        {
            let mut sink = sink.lock().expect("row sink");
            match item {
                Either::Left(done) => {
                    if sink.is_open() {
                        sink.end();
                    } else {
                        sink.affected(done.rows_affected());
                    }
                }
                Either::Right(row) => {
                    if !sink.is_open() {
                        sink.begin(
                            row.columns()
                                .iter()
                                .map(|c| ColumnMeta {
                                    name: c.name().to_owned(),
                                    type_name: c.type_info().name().to_owned(),
                                })
                                .collect(),
                        );
                    }
                    if sink.wants_row() {
                        sink.push(
                            (0..row.len())
                                .map(|i| match row.try_get_unchecked::<Option<String>, _>(i) {
                                    Ok(Some(text)) => Cell::Value(text),
                                    Ok(None) => Cell::Null,
                                    Err(_) => {
                                        match row.try_get_unchecked::<Option<Vec<u8>>, _>(i) {
                                            Ok(Some(bytes)) => Cell::binary(&bytes),
                                            _ => Cell::Null,
                                        }
                                    }
                                })
                                .collect(),
                        );
                    }
                }
            }
        }
        Ok(())
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
                         WHERE i.indrelid = c.oid AND i.indisprimary AND k.attnum = a.attnum),
                        (SELECT string_agg(e.enumlabel, chr(31) ORDER BY e.enumsortorder)
                         FROM pg_enum e WHERE e.enumtypid = a.atttypid),
                        (SELECT rn.nspname || chr(31) || rc.relname || chr(31) || ra.attname
                         FROM pg_constraint con
                         JOIN pg_class rc ON rc.oid = con.confrelid
                         JOIN pg_namespace rn ON rn.oid = rc.relnamespace
                         JOIN pg_attribute ra ON ra.attrelid = con.confrelid AND ra.attnum = con.confkey[1]
                         WHERE con.conrelid = c.oid AND con.contype = 'f'
                           AND cardinality(con.conkey) = 1 AND con.conkey[1] = a.attnum
                         LIMIT 1)
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
            // Neither engine will state its CREATE TABLE; a dump rebuilds it
            // from what is here, which is faithful for what is here.
            show_create: None,
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

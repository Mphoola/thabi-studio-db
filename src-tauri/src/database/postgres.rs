use super::{
    Capabilities, Cell, ColumnMeta, ConnectParams, DatabaseSession, Engine, QueryResult,
    ServerInfo, TlsMode, error, first_column,
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
    #[allow(dead_code)]
    backend_pid: i32,
    host: String,
    port: u16,
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
            },
            backend_pid: 0,
            host: params.host.clone(),
            port: params.port,
        };
        let row = session
            .execute(
                "SELECT current_setting('server_version'), pg_backend_pid()",
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
        Ok(session)
    }

    fn server_info(&self) -> &ServerInfo {
        &self.info
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            cancel_query: false,
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

    async fn close(self) -> Result<(), AppError> {
        self.connection
            .close()
            .await
            .map_err(|_| AppError::connection_close_failed())
    }
}

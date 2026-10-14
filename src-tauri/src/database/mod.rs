//! Database drivers behind one contract.
//!
//! Each engine has a `*Session` type that owns exactly one server session
//! (one TCP connection). Sessions are not pooled: statements such as `USE`,
//! `SET`, `BEGIN … COMMIT` and temp tables only work when consecutive
//! statements run on the same server session, and cancelling a query needs
//! to know which backend is running it.

pub mod error;
mod mysql;
mod postgres;
mod sqlserver;
mod values;

use crate::errors::AppError;
use serde::Serialize;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub use values::{Cell, ColumnMeta, QueryResult};

/// Upper bound for opening a session, including DNS and TLS.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Upper bound for a health check.
pub const PING_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    #[serde(rename = "mysql")]
    MySql,
    Postgres,
    #[serde(rename = "sqlserver")]
    SqlServer,
}

impl Engine {
    pub fn from_driver(driver: &str) -> Result<Self, AppError> {
        match driver {
            "mysql" => Ok(Self::MySql),
            "postgres" => Ok(Self::Postgres),
            "sqlserver" => Ok(Self::SqlServer),
            _ => Err(AppError::unsupported_driver()),
        }
    }
}

/// How the connection is encrypted. Never changed silently: the user picks
/// it per connection, and verification is the default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlsMode {
    /// Encrypt and verify the certificate and host name (default).
    Verify,
    /// Encrypt without verifying the certificate.
    EncryptOnly,
    /// No encryption.
    Off,
}

impl TlsMode {
    pub fn from_setting(value: &str) -> Result<Self, AppError> {
        match value {
            "verify-full" => Ok(Self::Verify),
            "require" => Ok(Self::EncryptOnly),
            "disable" => Ok(Self::Off),
            _ => Err(AppError::new(
                "tls_configuration",
                "This connection has an unknown TLS setting. Edit it and choose one.",
            )),
        }
    }
}

/// Everything needed to open a session. The password lives here only for
/// the duration of a connect call.
#[derive(Clone)]
pub struct ConnectParams {
    pub engine: Engine,
    pub host: String,
    pub port: u16,
    pub database: Option<String>,
    pub username: String,
    pub password: Option<String>,
    pub tls: TlsMode,
    /// Custom CA certificate (PEM) used with `TlsMode::Verify`.
    pub ca_path: Option<PathBuf>,
}

impl std::fmt::Debug for ConnectParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectParams")
            .field("engine", &self.engine)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("database", &self.database)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .field("tls", &self.tls)
            .field("ca_path", &self.ca_path)
            .finish()
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub engine: Engine,
    /// Human-readable product and version, e.g. "PostgreSQL 16.4".
    pub version: String,
}

/// What a driver can do reliably. The UI hides or explains anything false.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub cancel_query: bool,
    pub schemas: bool,
    pub rows_affected: bool,
    pub multiple_result_sets: bool,
}

/// The contract every engine implements. Methods take `&mut self` because a
/// session runs one statement at a time.
#[allow(async_fn_in_trait)]
pub trait DatabaseSession: Sized {
    async fn connect(params: &ConnectParams) -> Result<Self, AppError>;
    fn server_info(&self) -> &ServerInfo;
    fn capabilities(&self) -> Capabilities;
    async fn ping(&mut self) -> Result<(), AppError>;
    async fn list_databases(&mut self) -> Result<Vec<String>, AppError>;
    /// Runs user-authored SQL as written and keeps at most `max_rows` rows.
    async fn execute(&mut self, sql: &str, max_rows: usize) -> Result<QueryResult, AppError>;
    async fn close(self) -> Result<(), AppError>;
}

/// A session for any supported engine.
pub enum Session {
    MySql(mysql::MySqlSession),
    Postgres(postgres::PgSession),
    SqlServer(Box<sqlserver::SqlServerSession>),
}

macro_rules! dispatch {
    ($self:expr, $session:ident => $body:expr) => {
        match $self {
            Session::MySql($session) => $body,
            Session::Postgres($session) => $body,
            Session::SqlServer($session) => $body,
        }
    };
}

impl Session {
    /// Opens a session with an overall timeout. DNS is checked first so a
    /// typo in the host name gets its own message.
    pub async fn connect(params: &ConnectParams) -> Result<Self, AppError> {
        tokio::time::timeout(CONNECT_TIMEOUT, async {
            error::check_host_resolves(&params.host, params.port).await?;
            Ok(match params.engine {
                Engine::MySql => Self::MySql(mysql::MySqlSession::connect(params).await?),
                Engine::Postgres => Self::Postgres(postgres::PgSession::connect(params).await?),
                Engine::SqlServer => Self::SqlServer(Box::new(
                    sqlserver::SqlServerSession::connect(params).await?,
                )),
            })
        })
        .await
        .map_err(|_| error::timed_out(&params.host, params.port))?
    }

    pub fn server_info(&self) -> &ServerInfo {
        dispatch!(self, s => s.server_info())
    }

    pub fn capabilities(&self) -> Capabilities {
        dispatch!(self, s => s.capabilities())
    }

    pub async fn ping(&mut self) -> Result<(), AppError> {
        tokio::time::timeout(PING_TIMEOUT, async { dispatch!(self, s => s.ping().await) })
            .await
            .map_err(|_| {
                AppError::new(
                    "timeout",
                    "The server didn't answer a health check within 10 seconds.",
                )
            })?
    }

    pub async fn list_databases(&mut self) -> Result<Vec<String>, AppError> {
        dispatch!(self, s => s.list_databases().await)
    }

    pub async fn execute(&mut self, sql: &str, max_rows: usize) -> Result<QueryResult, AppError> {
        let started = Instant::now();
        let mut result = dispatch!(self, s => s.execute(sql, max_rows).await)?;
        result.elapsed_ms = started.elapsed().as_millis() as u64;
        Ok(result)
    }

    pub async fn close(self) -> Result<(), AppError> {
        match self {
            Session::MySql(s) => s.close().await,
            Session::Postgres(s) => s.close().await,
            Session::SqlServer(s) => s.close().await,
        }
    }
}

/// Result of a connection test.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub server: ServerInfo,
    pub elapsed_ms: u64,
}

/// Opens a session, checks it, closes it. Nothing is kept.
pub async fn test_connection(params: &ConnectParams) -> Result<TestResult, AppError> {
    let started = Instant::now();
    let mut session = Session::connect(params).await?;
    let ping = session.ping().await;
    let server = session.server_info().clone();
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let close = session.close().await;
    ping?;
    close?;
    Ok(TestResult { server, elapsed_ms })
}

/// First column of every row, as text. Used for metadata queries.
fn first_column(result: QueryResult) -> Vec<String> {
    result
        .rows
        .into_iter()
        .filter_map(|row| match row.into_iter().next() {
            Some(Cell::Value(value)) => Some(value),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_never_contains_the_password() {
        let params = ConnectParams {
            engine: Engine::Postgres,
            host: "localhost".into(),
            port: 5432,
            database: None,
            username: "app".into(),
            password: Some("hunter2".into()),
            tls: TlsMode::Verify,
            ca_path: None,
        };
        assert!(!format!("{params:?}").contains("hunter2"));
    }

    #[test]
    fn tls_settings_map_to_modes() {
        assert_eq!(
            TlsMode::from_setting("verify-full").unwrap(),
            TlsMode::Verify
        );
        assert_eq!(
            TlsMode::from_setting("require").unwrap(),
            TlsMode::EncryptOnly
        );
        assert_eq!(TlsMode::from_setting("disable").unwrap(), TlsMode::Off);
        assert!(TlsMode::from_setting("prefer").is_err());
    }
}

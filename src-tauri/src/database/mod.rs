//! Database drivers behind one contract.
//!
//! Each engine has a `*Session` type that owns exactly one server session
//! (one TCP connection). Sessions are not pooled: statements such as `USE`,
//! `SET`, `BEGIN … COMMIT` and temp tables only work when consecutive
//! statements run on the same server session, and cancelling a query needs
//! to know which backend is running it.

pub mod edit;
pub mod error;
pub mod filter;
pub mod metadata;
mod mysql;
mod postgres;
pub mod quote;
mod sqlserver;
mod values;

use crate::{
    errors::AppError,
    ssh::{SshParams, Tunnel},
};
use serde::Serialize;
use std::{
    net::SocketAddr,
    path::PathBuf,
    time::{Duration, Instant},
};

pub use metadata::{DbObject, ObjectKind, ObjectStructure, TextRows};
pub use values::{Cell, ColumnMeta, QueryResult};

/// Upper bound for opening a session, including DNS and TLS.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Upper bound when an SSH tunnel has to be opened first.
pub const TUNNEL_CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
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
    /// Reach the database through this SSH server.
    pub ssh: Option<SshParams>,
    /// Where to open the TCP connection when it differs from host:port
    /// (the local end of an SSH tunnel). Set by `Session::connect`.
    pub connect_to: Option<SocketAddr>,
    /// Ask the server to refuse writes for this session (safe mode
    /// "read-only"). The app checks too; this is the backstop for engines
    /// that can set it.
    pub read_only: bool,
}

/// The session statement that makes a session read-only, per engine. SQL
/// Server has no session equivalent, so it returns `None`.
fn read_only_statement(engine: Engine) -> Option<&'static str> {
    match engine {
        Engine::MySql => Some("SET SESSION TRANSACTION READ ONLY"),
        Engine::Postgres => Some("SET default_transaction_read_only = on"),
        Engine::SqlServer => None,
    }
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
            .field("ssh", &self.ssh)
            .field("read_only", &self.read_only)
            .finish()
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub engine: Engine,
    /// Human-readable product and version, e.g. "PostgreSQL 16.4".
    pub version: String,
    /// Database the session opened in, when the engine has one.
    pub database: Option<String>,
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
    /// Runs app-generated metadata SQL with bound text parameters.
    async fn query_text(&mut self, sql: &str, params: &[&str]) -> Result<TextRows, AppError>;
    /// Schemas in a database, or `None` when the engine has no schema level
    /// (MySQL, where a database is a schema).
    async fn list_schemas(&mut self, database: &str) -> Result<Option<Vec<String>>, AppError>;
    async fn list_objects(
        &mut self,
        database: &str,
        schema: &str,
    ) -> Result<Vec<DbObject>, AppError>;
    async fn describe(
        &mut self,
        database: &str,
        schema: &str,
        name: &str,
        kind: ObjectKind,
    ) -> Result<ObjectStructure, AppError>;
    /// SQL that reads the first `limit` rows of an object, fully quoted.
    fn preview_sql(&self, database: &str, schema: &str, name: &str, limit: usize) -> String;
    async fn close(self) -> Result<(), AppError>;
}

enum Driver {
    MySql(mysql::MySqlSession),
    Postgres(postgres::PgSession),
    SqlServer(Box<sqlserver::SqlServerSession>),
}

/// A session for any supported engine, plus the SSH tunnel it runs
/// through, if any. Closing the session closes the tunnel.
pub struct Session {
    driver: Driver,
    tunnel: Option<Tunnel>,
}

macro_rules! dispatch {
    ($self:expr, $session:ident => $body:expr) => {
        match &mut $self.driver {
            Driver::MySql($session) => $body,
            Driver::Postgres($session) => $body,
            Driver::SqlServer($session) => $body,
        }
    };
}

impl Driver {
    async fn connect(params: &ConnectParams) -> Result<Self, AppError> {
        Ok(match params.engine {
            Engine::MySql => Self::MySql(mysql::MySqlSession::connect(params).await?),
            Engine::Postgres => Self::Postgres(postgres::PgSession::connect(params).await?),
            Engine::SqlServer => Self::SqlServer(Box::new(
                sqlserver::SqlServerSession::connect(params).await?,
            )),
        })
    }
}

impl Session {
    /// Opens a session with an overall timeout, through an SSH tunnel when
    /// the params ask for one. Without a tunnel, DNS is checked first so a
    /// typo in the host name gets its own message. With a tunnel the
    /// database host only has to resolve on the SSH server.
    pub async fn connect(params: &ConnectParams) -> Result<Self, AppError> {
        let mut session = (if let Some(ssh) = &params.ssh {
            tokio::time::timeout(TUNNEL_CONNECT_TIMEOUT, async {
                let tunnel = crate::ssh::open_tunnel(ssh, &params.host, params.port).await?;
                let mut through = params.clone();
                through.connect_to = Some(tunnel.local_addr());
                match Driver::connect(&through).await {
                    Ok(driver) => Ok(Self {
                        driver,
                        tunnel: Some(tunnel),
                    }),
                    Err(error) => {
                        tunnel.close().await;
                        Err(error)
                    }
                }
            })
            .await
            .map_err(|_| error::timed_out(&params.host, params.port))?
        } else {
            tokio::time::timeout(CONNECT_TIMEOUT, async {
                error::check_host_resolves(&params.host, params.port).await?;
                Ok(Self {
                    driver: Driver::connect(params).await?,
                    tunnel: None,
                })
            })
            .await
            .map_err(|_| error::timed_out(&params.host, params.port))?
        })?;
        apply_read_only(&mut session, params).await?;
        Ok(session)
    }

    /// How another session can cancel this one's running statement.
    pub fn cancel_target(&self) -> Option<CancelTarget> {
        match &self.driver {
            Driver::Postgres(s) => Some(CancelTarget::Postgres(s.backend_pid())),
            Driver::MySql(s) => Some(CancelTarget::MySql(s.connection_id())),
            Driver::SqlServer(_) => None,
        }
    }

    /// Local end of the SSH tunnel, if this session uses one.
    pub fn tunnel_addr(&self) -> Option<SocketAddr> {
        self.tunnel.as_ref().map(Tunnel::local_addr)
    }

    pub fn server_info(&self) -> &ServerInfo {
        match &self.driver {
            Driver::MySql(s) => s.server_info(),
            Driver::Postgres(s) => s.server_info(),
            Driver::SqlServer(s) => s.server_info(),
        }
    }

    pub fn capabilities(&self) -> Capabilities {
        match &self.driver {
            Driver::MySql(s) => s.capabilities(),
            Driver::Postgres(s) => s.capabilities(),
            Driver::SqlServer(s) => s.capabilities(),
        }
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

    pub async fn list_schemas(&mut self, database: &str) -> Result<Option<Vec<String>>, AppError> {
        dispatch!(self, s => s.list_schemas(database).await)
    }

    pub async fn list_objects(
        &mut self,
        database: &str,
        schema: &str,
    ) -> Result<Vec<DbObject>, AppError> {
        dispatch!(self, s => s.list_objects(database, schema).await)
    }

    pub async fn describe(
        &mut self,
        database: &str,
        schema: &str,
        name: &str,
        kind: ObjectKind,
    ) -> Result<ObjectStructure, AppError> {
        dispatch!(self, s => s.describe(database, schema, name, kind).await)
    }

    /// First `limit` rows of a table or view, read-only.
    pub async fn preview(
        &mut self,
        database: &str,
        schema: &str,
        name: &str,
        limit: usize,
    ) -> Result<QueryResult, AppError> {
        let sql = dispatch!(self, s => s.preview_sql(database, schema, name, limit));
        self.execute(&sql, limit).await
    }

    pub async fn close(self) -> Result<(), AppError> {
        let result = match self.driver {
            Driver::MySql(s) => s.close().await,
            Driver::Postgres(s) => s.close().await,
            Driver::SqlServer(s) => s.close().await,
        };
        if let Some(tunnel) = self.tunnel {
            tunnel.close().await;
        }
        result
    }
}

/// Identifies a session's server-side process so another session can stop
/// its running statement.
#[derive(Clone, Copy, Debug)]
pub enum CancelTarget {
    Postgres(i32),
    MySql(u64),
}

/// Asks the server to stop the statement running on `target`, from a new,
/// short-lived session. Best effort; the caller falls back to dropping the
/// session if this doesn't work.
pub async fn cancel_backend(params: &ConnectParams, target: CancelTarget) -> Result<(), AppError> {
    let sql = match target {
        CancelTarget::Postgres(pid) => format!("SELECT pg_cancel_backend({pid})"),
        CancelTarget::MySql(id) => format!("KILL QUERY {id}"),
    };
    let mut session = Session::connect(params).await?;
    let result = session.execute(&sql, 1).await.map(|_| ());
    let _ = session.close().await;
    result
}

/// Runs the engine's read-only session statement when the connection asks
/// for one. SQL Server has no session equivalent, so it is skipped; the app
/// check is the only guard there and the UI says so.
async fn apply_read_only(session: &mut Session, params: &ConnectParams) -> Result<(), AppError> {
    if !params.read_only {
        return Ok(());
    }
    match read_only_statement(params.engine) {
        Some(sql) => session.execute(sql, 1).await.map(|_| ()),
        None => Ok(()),
    }
}

/// SQL that reads up to `limit` rows of a table or view, fully quoted.
/// `filter` is a WHERE clause from `filter::where_clause` ("" for none).
pub fn object_data_sql(
    engine: Engine,
    database: &str,
    schema: &str,
    name: &str,
    filter: &str,
    limit: usize,
) -> String {
    let table = quote::table(engine, database, schema, name);
    match engine {
        Engine::SqlServer => format!("SELECT TOP ({limit}) * FROM {table}{filter}"),
        _ => format!("SELECT * FROM {table}{filter} LIMIT {limit}"),
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
            ssh: None,
            connect_to: None,
            read_only: false,
        };
        assert!(!format!("{params:?}").contains("hunter2"));
    }

    #[test]
    fn read_only_statement_covers_engines_that_can() {
        assert!(read_only_statement(Engine::MySql).is_some());
        assert!(read_only_statement(Engine::Postgres).is_some());
        assert_eq!(read_only_statement(Engine::SqlServer), None);
    }

    #[test]
    fn object_data_sql_quotes_names_for_each_engine() {
        assert_eq!(
            object_data_sql(Engine::Postgres, "shop", "Sales Ü", "Order Details", "", 10),
            "SELECT * FROM \"Sales Ü\".\"Order Details\" LIMIT 10"
        );
        assert_eq!(
            object_data_sql(Engine::MySql, "shop", "", "a`b", "", 5),
            "SELECT * FROM `shop`.`a``b` LIMIT 5"
        );
        assert_eq!(
            object_data_sql(Engine::SqlServer, "shop", "dbo", "x]y", " WHERE 1 = 1", 5),
            "SELECT TOP (5) * FROM [shop].[dbo].[x]]y] WHERE 1 = 1"
        );
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

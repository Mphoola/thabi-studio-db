use crate::{commands::connection_profiles::ConnectionProfile, errors::AppError};
use sqlx::{
    mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions, MySqlSslMode},
    postgres::{PgConnectOptions, PgPool, PgPoolOptions, PgSslMode},
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tiberius::{AuthMethod, Client, Config, EncryptionLevel};
use tokio::{net::TcpStream, sync::Mutex, time::timeout};
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const PING_TIMEOUT: Duration = Duration::from_secs(10);

type SqlServerClient = Client<Compat<TcpStream>>;

enum DriverConnection {
    MySql(MySqlPool),
    Postgres(PgPool),
    SqlServer(Box<SqlServerClient>),
}

impl DriverConnection {
    async fn open(profile: &ConnectionProfile, password: Option<&str>) -> Result<Self, AppError> {
        if profile.ssh_enabled {
            return Err(AppError::unsupported_ssh());
        }
        if profile.tls_mode != "verify-full" {
            return Err(AppError::unsupported_tls_mode());
        }

        match profile.driver.as_str() {
            "mysql" => {
                let mut options = MySqlConnectOptions::new()
                    .host(&profile.host)
                    .port(profile.port)
                    .username(&profile.username)
                    .password(password.unwrap_or_default())
                    .ssl_mode(MySqlSslMode::VerifyIdentity);
                if !profile.database_name.is_empty() {
                    options = options.database(&profile.database_name);
                }
                let pool = timeout(
                    CONNECT_TIMEOUT,
                    MySqlPoolOptions::new()
                        .max_connections(5)
                        .acquire_timeout(CONNECT_TIMEOUT)
                        .connect_with(options),
                )
                .await
                .map_err(|_| AppError::connection_timeout())?
                .map_err(map_sqlx_error)?;
                Ok(Self::MySql(pool))
            }
            "postgres" => {
                let mut options = PgConnectOptions::new()
                    .host(&profile.host)
                    .port(profile.port)
                    .username(&profile.username)
                    .password(password.unwrap_or_default())
                    .ssl_mode(PgSslMode::VerifyFull);
                if !profile.database_name.is_empty() {
                    options = options.database(&profile.database_name);
                }
                let pool = timeout(
                    CONNECT_TIMEOUT,
                    PgPoolOptions::new()
                        .max_connections(5)
                        .acquire_timeout(CONNECT_TIMEOUT)
                        .connect_with(options),
                )
                .await
                .map_err(|_| AppError::connection_timeout())?
                .map_err(map_sqlx_error)?;
                Ok(Self::Postgres(pool))
            }
            "sqlserver" => {
                let mut config = Config::new();
                config.host(&profile.host);
                config.port(profile.port);
                config.authentication(AuthMethod::sql_server(
                    &profile.username,
                    password.unwrap_or_default(),
                ));
                config.encryption(EncryptionLevel::Required);
                if !profile.database_name.is_empty() {
                    config.database(&profile.database_name);
                }

                let result = timeout(CONNECT_TIMEOUT, async {
                    let tcp = TcpStream::connect((profile.host.as_str(), profile.port))
                        .await
                        .map_err(|_| AppError::connection_failed())?;
                    tcp.set_nodelay(true)
                        .map_err(|_| AppError::connection_failed())?;
                    Client::connect(config, tcp.compat_write())
                        .await
                        .map_err(map_tiberius_error)
                })
                .await
                .map_err(|_| AppError::connection_timeout())??;
                Ok(Self::SqlServer(Box::new(result)))
            }
            _ => Err(AppError::unsupported_driver()),
        }
    }

    async fn ping(&mut self) -> Result<(), AppError> {
        timeout(PING_TIMEOUT, async {
            match self {
                Self::MySql(pool) => sqlx::query("SELECT 1")
                    .execute(&*pool)
                    .await
                    .map(|_| ())
                    .map_err(map_sqlx_error),
                Self::Postgres(pool) => sqlx::query("SELECT 1")
                    .execute(&*pool)
                    .await
                    .map(|_| ())
                    .map_err(map_sqlx_error),
                Self::SqlServer(client) => client
                    .simple_query("SELECT 1")
                    .await
                    .map_err(map_tiberius_error)?
                    .into_row()
                    .await
                    .and_then(|row| {
                        row.ok_or_else(|| {
                            tiberius::error::Error::Protocol("Health check returned no row".into())
                        })
                    })
                    .map(|_| ())
                    .map_err(map_tiberius_error),
            }
        })
        .await
        .map_err(|_| AppError::connection_timeout())?
    }

    async fn close(self) -> Result<(), AppError> {
        match self {
            Self::MySql(pool) => {
                pool.close().await;
                Ok(())
            }
            Self::Postgres(pool) => {
                pool.close().await;
                Ok(())
            }
            Self::SqlServer(client) => (*client).close().await,
        }
        .map_err(|_| AppError::connection_close_failed())
    }
}

fn map_sqlx_error(error: sqlx::Error) -> AppError {
    match error {
        sqlx::Error::Tls(_) => AppError::tls_verification_failed(),
        sqlx::Error::Database(_) => AppError::authentication_failed(),
        _ => AppError::connection_failed(),
    }
}

fn map_tiberius_error(error: tiberius::error::Error) -> AppError {
    if matches!(error, tiberius::error::Error::Tls(_)) {
        return AppError::tls_verification_failed();
    }
    if matches!(error.code(), Some(18456 | 4060)) {
        return AppError::authentication_failed();
    }
    AppError::connection_failed()
}

enum ActiveConnection {
    Connecting,
    Connected(Arc<Mutex<DriverConnection>>),
}

#[derive(Default)]
pub struct ConnectionService {
    active: Mutex<HashMap<String, ActiveConnection>>,
}

struct ConnectionReservation<'a> {
    service: &'a ConnectionService,
    profile_id: String,
    committed: bool,
}

impl ConnectionReservation<'_> {
    fn commit(&mut self) {
        self.committed = true;
    }
}

impl Drop for ConnectionReservation<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Ok(mut active) = self.service.active.try_lock()
            && matches!(
                active.get(&self.profile_id),
                Some(ActiveConnection::Connecting)
            )
        {
            active.remove(&self.profile_id);
        }
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatus {
    pub profile_id: String,
    pub connection_id: String,
    pub status: &'static str,
}

impl ConnectionService {
    pub async fn test(
        &self,
        profile: &ConnectionProfile,
        password: Option<&str>,
    ) -> Result<(), AppError> {
        let mut connection = DriverConnection::open(profile, password).await?;
        let ping_result = connection.ping().await;
        let close_result = connection.close().await;
        ping_result?;
        close_result
    }

    pub async fn connect(
        &self,
        profile: &ConnectionProfile,
        password: Option<&str>,
    ) -> Result<ConnectionStatus, AppError> {
        {
            let mut active = self.active.lock().await;
            match active.get(&profile.id) {
                Some(ActiveConnection::Connected(_)) => {
                    return Ok(ConnectionStatus {
                        profile_id: profile.id.clone(),
                        connection_id: profile.id.clone(),
                        status: "connected",
                    });
                }
                Some(ActiveConnection::Connecting) => {
                    return Err(AppError::connection_in_progress());
                }
                None => {
                    active.insert(profile.id.clone(), ActiveConnection::Connecting);
                }
            }
        }
        let mut reservation = ConnectionReservation {
            service: self,
            profile_id: profile.id.clone(),
            committed: false,
        };

        let result = async {
            let mut driver = DriverConnection::open(profile, password).await?;
            driver.ping().await?;
            Ok::<_, AppError>(driver)
        }
        .await;

        let mut active = self.active.lock().await;
        let result = match result {
            Ok(driver) => {
                active.insert(
                    profile.id.clone(),
                    ActiveConnection::Connected(Arc::new(Mutex::new(driver))),
                );
                Ok(ConnectionStatus {
                    profile_id: profile.id.clone(),
                    connection_id: profile.id.clone(),
                    status: "connected",
                })
            }
            Err(error) => {
                active.remove(&profile.id);
                Err(error)
            }
        };
        reservation.commit();
        result
    }

    pub async fn disconnect_profile(&self, profile_id: &str) -> Result<(), AppError> {
        let connection = {
            let mut active = self.active.lock().await;
            match active.remove(profile_id) {
                Some(ActiveConnection::Connecting) => {
                    active.insert(profile_id.to_owned(), ActiveConnection::Connecting);
                    return Err(AppError::connection_in_progress());
                }
                Some(ActiveConnection::Connected(connection)) => Some(connection),
                None => None,
            }
        };
        if let Some(connection) = connection {
            let connection =
                Arc::try_unwrap(connection).map_err(|_| AppError::connection_close_failed())?;
            connection.into_inner().close().await?;
        }
        Ok(())
    }

    pub async fn connected_profiles(&self) -> Vec<String> {
        self.active
            .lock()
            .await
            .iter()
            .filter(|(_, status)| matches!(status, ActiveConnection::Connected(_)))
            .map(|(id, _)| id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> ConnectionProfile {
        ConnectionProfile {
            id: "profile-id".into(),
            name: "Test".into(),
            environment: "testing".into(),
            driver: "postgres".into(),
            host: "localhost".into(),
            port: 5432,
            database_name: "test".into(),
            username: "user".into(),
            auth_mode: "password".into(),
            tls_mode: "verify-full".into(),
            ssh_enabled: false,
            ssh_host: String::new(),
            ssh_port: 22,
            ssh_username: String::new(),
            ssh_auth_mode: "password".into(),
            has_database_password: false,
            has_ssh_password: false,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[tokio::test]
    async fn refuses_ssh_instead_of_falling_back_to_direct_connection() {
        let mut profile = profile();
        profile.ssh_enabled = true;

        let error = match DriverConnection::open(&profile, None).await {
            Ok(_) => panic!("SSH profile must not fall back to a direct connection"),
            Err(error) => error,
        };

        assert_eq!(error.kind, "ssh_unavailable");
    }

    #[tokio::test]
    async fn refuses_unverified_tls_modes() {
        let mut profile = profile();
        profile.tls_mode = "disabled".into();

        let error = match DriverConnection::open(&profile, None).await {
            Ok(_) => panic!("unverified TLS mode must not be accepted"),
            Err(error) => error,
        };

        assert_eq!(error.kind, "tls_configuration");
    }

    #[tokio::test]
    async fn refuses_unknown_database_drivers() {
        let mut profile = profile();
        profile.driver = "unknown".into();

        let error = match DriverConnection::open(&profile, None).await {
            Ok(_) => panic!("unknown database driver must not be accepted"),
            Err(error) => error,
        };

        assert_eq!(error.kind, "unsupported_driver");
    }

    #[tokio::test]
    async fn failed_connect_does_not_leave_a_profile_in_connecting_state() {
        let service = ConnectionService::default();
        let mut profile = profile();
        profile.driver = "unknown".into();

        assert_eq!(
            service.connect(&profile, None).await.unwrap_err().kind,
            "unsupported_driver"
        );
        assert!(service.connected_profiles().await.is_empty());
    }

    #[test]
    fn sql_server_tls_errors_are_sanitized() {
        let error = map_tiberius_error(tiberius::error::Error::Tls(
            "private certificate detail".into(),
        ));

        assert_eq!(error.kind, "tls");
        assert!(!error.message.contains("private certificate detail"));
    }
}

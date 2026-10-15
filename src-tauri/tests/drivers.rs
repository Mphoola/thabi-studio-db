//! Driver integration tests against real databases.
//!
//! These are `#[ignore]`d so `cargo test` stays offline. Run them with
//! `npm run test:integration`, which starts MySQL, PostgreSQL and SQL Server
//! in Docker and passes their throwaway credentials through the environment.

use thabi_studio_lib::database::{Cell, ConnectParams, Engine, Session, TlsMode, test_connection};

fn env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} is not set; run npm run test:integration"))
}

fn params(engine: Engine, tls: TlsMode) -> ConnectParams {
    let (port, username, database) = match engine {
        Engine::Postgres => (env("THABI_IT_PG_PORT"), "thabi", Some("thabi_it")),
        Engine::MySql => (env("THABI_IT_MYSQL_PORT"), "thabi", Some("thabi_it")),
        Engine::SqlServer => (env("THABI_IT_MSSQL_PORT"), "sa", Some("master")),
    };
    ConnectParams {
        engine,
        host: "127.0.0.1".into(),
        port: port.parse().unwrap(),
        database: database.map(str::to_owned),
        username: username.into(),
        password: Some(env("THABI_IT_PASSWORD")),
        tls,
        ca_path: None,
        ssh: None,
        connect_to: None,
    }
}

/// The TLS mode each local container accepts: Postgres has no TLS; MySQL and
/// SQL Server use self-signed certificates.
fn working_tls(engine: Engine) -> TlsMode {
    match engine {
        Engine::Postgres => TlsMode::Off,
        _ => TlsMode::EncryptOnly,
    }
}

fn values(result: &thabi_studio_lib::database::QueryResult) -> Vec<Vec<Option<String>>> {
    result
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| match cell {
                    Cell::Null => None,
                    Cell::Value(v) => Some(v.clone()),
                    Cell::Binary { preview, .. } => Some(format!("0x{preview}")),
                })
                .collect()
        })
        .collect()
}

async fn open(engine: Engine) -> Session {
    Session::connect(&params(engine, working_tls(engine)))
        .await
        .unwrap_or_else(|e| panic!("{engine:?} connect failed: {} {:?}", e.message, e.detail))
}

async fn connects_and_reports_version(engine: Engine, product: &str) {
    let result = test_connection(&params(engine, working_tls(engine)))
        .await
        .unwrap();
    assert!(
        result.server.version.starts_with(product),
        "unexpected version {}",
        result.server.version
    );
}

async fn reads_values_without_loss(engine: Engine, sql: &str) {
    let mut session = open(engine).await;
    let result = session.execute(sql, 100).await.unwrap();
    assert_eq!(
        values(&result),
        vec![vec![
            Some("9007199254740993".into()),
            Some("12345678901234567890.123456789".into()),
            None,
            Some("Zoë 日本".into()),
        ]]
    );
    assert_eq!(result.columns.len(), 4);
    session.close().await.unwrap();
}

async fn keeps_session_state_between_statements(engine: Engine, setup: &str, read: &str) {
    let mut session = open(engine).await;
    session.execute(setup, 10).await.unwrap();
    let result = session.execute(read, 10).await.unwrap();
    assert_eq!(values(&result), vec![vec![Some("42".into())]]);
    session.close().await.unwrap();
}

async fn bounds_rows_and_flags_truncation(engine: Engine, sql: &str) {
    let mut session = open(engine).await;
    let result = session.execute(sql, 5).await.unwrap();
    assert_eq!(result.rows.len(), 5);
    assert!(result.truncated);
    // The session is still usable afterwards.
    session.ping().await.unwrap();
    session.close().await.unwrap();
}

async fn reports_server_errors_with_detail(engine: Engine) {
    let mut session = open(engine).await;
    let error = session
        .execute("SELECT * FROM no_such_table_xyz", 10)
        .await
        .unwrap_err();
    assert_eq!(error.kind, "server");
    assert!(
        error
            .detail
            .unwrap()
            .to_lowercase()
            .contains("no_such_table_xyz")
    );
    session.ping().await.unwrap();
    session.close().await.unwrap();
}

async fn wrong_password_is_an_authentication_error(engine: Engine) {
    let mut p = params(engine, working_tls(engine));
    p.password = Some("definitely-wrong".into());
    let error = Session::connect(&p).await.err().unwrap();
    assert_eq!(
        error.kind, "authentication",
        "{} {:?}",
        error.message, error.detail
    );
    assert!(!format!("{error:?}").contains(&env("THABI_IT_PASSWORD")));
}

async fn unknown_database_is_named(engine: Engine) {
    let mut p = params(engine, working_tls(engine));
    p.database = Some("no_such_database_xyz".into());
    let error = Session::connect(&p).await.err().unwrap();
    assert_eq!(
        error.kind, "unknown_database",
        "{} {:?}",
        error.message, error.detail
    );
    assert!(error.message.contains("no_such_database_xyz"));
}

async fn closed_port_is_refused(engine: Engine) {
    let mut p = params(engine, working_tls(engine));
    p.port = 1;
    let error = Session::connect(&p).await.err().unwrap();
    assert_eq!(error.kind, "connection_refused");
}

async fn lists_databases(engine: Engine, expected: &str) {
    let mut session = open(engine).await;
    let databases = session.list_databases().await.unwrap();
    assert!(databases.iter().any(|d| d == expected), "{databases:?}");
    session.close().await.unwrap();
}

mod postgres {
    use super::*;
    const E: Engine = Engine::Postgres;

    #[tokio::test]
    #[ignore]
    async fn connects() {
        connects_and_reports_version(E, "PostgreSQL 16").await;
    }

    #[tokio::test]
    #[ignore]
    async fn values() {
        reads_values_without_loss(
            E,
            "SELECT 9007199254740993::int8, 12345678901234567890.123456789::numeric, NULL::text, 'Zoë 日本'",
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn session_state() {
        keeps_session_state_between_statements(
            E,
            "CREATE TEMP TABLE t (v int); INSERT INTO t VALUES (42)",
            "SELECT v FROM t",
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn truncation() {
        bounds_rows_and_flags_truncation(E, "SELECT generate_series(1, 1000)").await;
    }

    #[tokio::test]
    #[ignore]
    async fn rows_affected() {
        let mut session = open(E).await;
        let result = session
            .execute(
                "CREATE TEMP TABLE r (v int); INSERT INTO r VALUES (1), (2), (3)",
                10,
            )
            .await
            .unwrap();
        assert_eq!(result.rows_affected, Some(3));
        session.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn errors() {
        reports_server_errors_with_detail(E).await;
        wrong_password_is_an_authentication_error(E).await;
        unknown_database_is_named(E).await;
        closed_port_is_refused(E).await;
    }

    #[tokio::test]
    #[ignore]
    async fn tls_verification_reports_missing_tls() {
        let error = Session::connect(&params(E, TlsMode::Verify))
            .await
            .err()
            .unwrap();
        assert_eq!(
            error.kind, "tls_unsupported",
            "{} {:?}",
            error.message, error.detail
        );
    }

    #[tokio::test]
    #[ignore]
    async fn databases() {
        lists_databases(E, "thabi_it").await;
    }
}

mod mysql {
    use super::*;
    const E: Engine = Engine::MySql;

    #[tokio::test]
    #[ignore]
    async fn connects() {
        connects_and_reports_version(E, "MySQL 8.4").await;
    }

    #[tokio::test]
    #[ignore]
    async fn connects_without_tls() {
        connects_and_reports_version_with(E, TlsMode::Off).await;
    }

    #[tokio::test]
    #[ignore]
    async fn values() {
        reads_values_without_loss(
            E,
            "SELECT CAST(9007199254740993 AS SIGNED), CAST('12345678901234567890.123456789' AS DECIMAL(38,9)), NULL, 'Zoë 日本'",
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn session_state() {
        keeps_session_state_between_statements(E, "SET @answer = 42", "SELECT @answer").await;
    }

    #[tokio::test]
    #[ignore]
    async fn truncation() {
        bounds_rows_and_flags_truncation(
            E,
            "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 1000) SELECT i FROM n",
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn errors() {
        reports_server_errors_with_detail(E).await;
        wrong_password_is_an_authentication_error(E).await;
        unknown_database_is_named(E).await;
        closed_port_is_refused(E).await;
    }

    #[tokio::test]
    #[ignore]
    async fn self_signed_certificate_fails_verification() {
        let error = Session::connect(&params(E, TlsMode::Verify))
            .await
            .err()
            .unwrap();
        assert_eq!(error.kind, "tls", "{} {:?}", error.message, error.detail);
    }

    #[tokio::test]
    #[ignore]
    async fn databases() {
        lists_databases(E, "thabi_it").await;
    }
}

mod sqlserver {
    use super::*;
    const E: Engine = Engine::SqlServer;

    #[tokio::test]
    #[ignore]
    async fn connects() {
        connects_and_reports_version(E, "Microsoft SQL Server 2022").await;
    }

    #[tokio::test]
    #[ignore]
    async fn values() {
        reads_values_without_loss(
            E,
            "SELECT CAST(9007199254740993 AS BIGINT), CAST('12345678901234567890.123456789' AS DECIMAL(38,9)), CAST(NULL AS NVARCHAR(10)), N'Zoë 日本'",
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn session_state() {
        keeps_session_state_between_statements(
            E,
            "CREATE TABLE #t (v int); INSERT INTO #t VALUES (42)",
            "SELECT v FROM #t",
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn truncation() {
        bounds_rows_and_flags_truncation(
            E,
            "SELECT TOP 1000 ROW_NUMBER() OVER (ORDER BY (SELECT NULL)) FROM sys.all_objects",
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn errors() {
        reports_server_errors_with_detail(E).await;
        wrong_password_is_an_authentication_error(E).await;
        unknown_database_is_named(E).await;
        closed_port_is_refused(E).await;
    }

    #[tokio::test]
    #[ignore]
    async fn self_signed_certificate_fails_verification() {
        let error = Session::connect(&params(E, TlsMode::Verify))
            .await
            .err()
            .unwrap();
        assert_eq!(error.kind, "tls", "{} {:?}", error.message, error.detail);
    }

    #[tokio::test]
    #[ignore]
    async fn databases() {
        lists_databases(E, "master").await;
    }
}

async fn connects_and_reports_version_with(engine: Engine, tls: TlsMode) {
    let result = test_connection(&params(engine, tls)).await.unwrap();
    assert!(!result.server.version.is_empty());
}

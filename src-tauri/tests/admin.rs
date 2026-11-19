//! Server activity, kill, server facts and storage, against real databases.
//! `#[ignore]`d; run with `npm run test:integration`.

use std::time::{Duration, Instant};
use thabi_studio_lib::database::admin::{kill, overview, processes, storage};
use thabi_studio_lib::database::{Cell, ConnectParams, Engine, Session, TlsMode};

fn env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} is not set; run npm run test:integration"))
}

async fn open(engine: Engine) -> Session {
    let (port, username, database) = match engine {
        Engine::Postgres => (env("THABI_IT_PG_PORT"), "thabi", "thabi_it"),
        Engine::MySql => (env("THABI_IT_MYSQL_PORT"), "root", "thabi_it"),
        Engine::SqlServer => (env("THABI_IT_MSSQL_PORT"), "sa", "master"),
    };
    Session::connect(&ConnectParams {
        engine,
        host: "127.0.0.1".into(),
        port: port.parse().unwrap(),
        database: Some(database.into()),
        username: username.into(),
        password: Some(env("THABI_IT_PASSWORD")),
        tls: match engine {
            Engine::Postgres => TlsMode::Off,
            _ => TlsMode::EncryptOnly,
        },
        ca_path: None,
        client_cert: None,
        client_key: None,
        ssh: None,
        connect_to: None,
        read_only: false,
        startup_commands: Vec::new(),
    })
    .await
    .unwrap_or_else(|e| panic!("{engine:?} connect failed: {}", e.message))
}

fn first(result: &thabi_studio_lib::database::QueryResult) -> String {
    match result.rows.first().and_then(|row| row.first()) {
        Some(Cell::Value(v)) => v.clone(),
        other => panic!("no value: {other:?}"),
    }
}

/// A second session sleeps; the first finds it and stops it.
async fn finds_and_stops_a_session(engine: Engine) {
    let mut watcher = open(engine).await;
    let mut sleeper = open(engine).await;
    let id_sql = match engine {
        Engine::Postgres => "SELECT pg_backend_pid()::text",
        Engine::MySql => "SELECT CONNECTION_ID()",
        Engine::SqlServer => "SELECT CAST(@@SPID AS nvarchar(10))",
    };
    let id = first(&sleeper.execute(id_sql, 1).await.unwrap());
    let sleep = match engine {
        Engine::Postgres => "SELECT pg_sleep(30)",
        Engine::MySql => "SELECT SLEEP(30)",
        Engine::SqlServer => "WAITFOR DELAY '00:00:30'",
    };
    let started = Instant::now();
    let running = tokio::spawn(async move { sleeper.execute(sleep, 1).await });
    tokio::time::sleep(Duration::from_millis(700)).await;

    let listed = processes(&mut watcher).await.expect("process list");
    let found = listed
        .iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("{engine:?} did not list {id}: {listed:?}"));
    assert!(!found.own);
    assert!(
        listed.iter().any(|p| p.own),
        "{engine:?} did not mark its own session"
    );

    // Cancel where the engine can; SQL Server can only end the session.
    kill(&mut watcher, &id, engine == Engine::SqlServer)
        .await
        .unwrap_or_else(|e| panic!("{engine:?} kill: {} {:?}", e.message, e.detail));
    let outcome = tokio::time::timeout(Duration::from_secs(10), running)
        .await
        .expect("the sleep should stop")
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(20));
    // MySQL's SLEEP() returns 1 when interrupted instead of failing.
    if engine != Engine::MySql {
        assert!(outcome.is_err(), "{engine:?} sleep was not stopped");
    }
    assert!(kill(&mut watcher, "not a number", true).await.is_err());
}

async fn reports_the_server_and_its_tables(engine: Engine) {
    let mut session = open(engine).await;
    let facts = overview(&mut session).await.expect("overview");
    let version = &facts.iter().find(|f| f.label == "Version").unwrap().value;
    assert!(!version.is_empty(), "{engine:?} has no version");
    assert!(
        facts
            .iter()
            .all(|f| f.label != "Up for" || !f.value.is_empty())
    );

    let (db, schema, create) = match engine {
        Engine::Postgres => (
            "thabi_it",
            "public",
            "CREATE TABLE thabi_sized (id int, note text)",
        ),
        Engine::MySql => (
            "thabi_it",
            "",
            "CREATE TABLE thabi_sized (id int, note text)",
        ),
        Engine::SqlServer => (
            "master",
            "dbo",
            "CREATE TABLE dbo.thabi_sized (id int, note nvarchar(100))",
        ),
    };
    let _ = session.execute("DROP TABLE IF EXISTS thabi_sized", 0).await;
    session.execute(create, 0).await.unwrap();
    session
        .execute("INSERT INTO thabi_sized VALUES (1, 'a'), (2, 'b')", 0)
        .await
        .unwrap();
    let sizes = storage(&mut session, db, schema).await.expect("storage");
    let table = sizes
        .iter()
        .find(|t| t.name == "thabi_sized")
        .unwrap_or_else(|| panic!("{engine:?} did not list the table: {sizes:?}"));
    assert!(table.total_bytes.unwrap_or(0) > 0, "{engine:?}: {table:?}");
    session.execute("DROP TABLE thabi_sized", 0).await.unwrap();
}

macro_rules! engine_tests {
    ($module:ident, $engine:expr) => {
        mod $module {
            use super::*;

            #[tokio::test]
            #[ignore]
            async fn finds_and_stops_a_session() {
                super::finds_and_stops_a_session($engine).await;
            }

            #[tokio::test]
            #[ignore]
            async fn reports_the_server_and_its_tables() {
                super::reports_the_server_and_its_tables($engine).await;
            }
        }
    };
}

engine_tests!(mysql, Engine::MySql);
engine_tests!(postgres, Engine::Postgres);
engine_tests!(sqlserver, Engine::SqlServer);

/// A query with parameters returns numbers, dates and text as values on
/// MySQL, whose prepared protocol sends them in binary rather than as text.
#[tokio::test]
#[ignore]
async fn mysql_bound_results_decode_by_type() {
    use thabi_studio_lib::database::values::Param;
    let mut session = open(Engine::MySql).await;
    let result = session
        .execute_bound(
            "SELECT ? + 1, CAST(? AS UNSIGNED), CAST('2026-10-20 08:30:00' AS DATETIME), \
             CAST('2026-10-20' AS DATE), 1.25, CAST(2.5 AS DOUBLE), 'naïve', NULL",
            &[Param::Number("41".into()), Param::Number("7".into())],
            1,
        )
        .await
        .expect("bound select");
    let cells: Vec<String> = result.rows[0]
        .iter()
        .map(|cell| match cell {
            Cell::Value(v) => v.clone(),
            Cell::Null => "NULL".into(),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        cells,
        [
            "42",
            "7",
            "2026-10-20 08:30:00",
            "2026-10-20",
            "1.25",
            "2.5",
            "naïve",
            "NULL"
        ]
    );
}

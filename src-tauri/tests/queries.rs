//! Query-tab integration tests: running, cancelling and timeouts on real
//! databases. Run with `npm run test:integration`.

use std::time::{Duration, Instant};
use thabi_studio_lib::{
    database::{ConnectParams, Engine, TlsMode},
    queries::QueryTab,
};

fn env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} is not set; run npm run test:integration"))
}

fn params(engine: Engine) -> ConnectParams {
    let (port, username, database, tls) = match engine {
        Engine::Postgres => (env("THABI_IT_PG_PORT"), "thabi", "thabi_it", TlsMode::Off),
        Engine::MySql => (
            env("THABI_IT_MYSQL_PORT"),
            "thabi",
            "thabi_it",
            TlsMode::EncryptOnly,
        ),
        Engine::SqlServer => (
            env("THABI_IT_MSSQL_PORT"),
            "sa",
            "master",
            TlsMode::EncryptOnly,
        ),
    };
    ConnectParams {
        engine,
        host: "127.0.0.1".into(),
        port: port.parse().unwrap(),
        database: Some(database.into()),
        username: username.into(),
        password: Some(env("THABI_IT_PASSWORD")),
        tls,
        ca_path: None,
        ssh: None,
        connect_to: None,
        read_only: false,
    }
}

/// Starts `sql` on the tab, cancels it after a moment, and returns the
/// outcome and how long it took.
async fn run_and_cancel(
    engine: Engine,
    tab: &std::sync::Arc<QueryTab>,
    sql: &'static str,
) -> (
    Result<thabi_studio_lib::database::QueryResult, thabi_studio_lib::AppError>,
    Duration,
) {
    let p = params(engine);
    let started = Instant::now();
    let runner = {
        let tab = tab.clone();
        let p = p.clone();
        tokio::spawn(async move { tab.run(&p, None, sql, 100, None).await })
    };
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert!(tab.is_running(), "{engine:?} query should still be running");
    assert!(tab.cancel(&p));
    let outcome = runner.await.unwrap();
    (outcome, started.elapsed())
}

#[tokio::test]
#[ignore]
async fn postgres_cancel_keeps_the_session() {
    let tab = std::sync::Arc::new(QueryTab::default());
    let p = params(Engine::Postgres);
    tab.run(
        &p,
        None,
        "CREATE TEMP TABLE kept (v int); INSERT INTO kept VALUES (7)",
        10,
        None,
    )
    .await
    .unwrap();

    let (outcome, took) = run_and_cancel(Engine::Postgres, &tab, "SELECT pg_sleep(30)").await;
    let error = outcome.unwrap_err();
    assert_eq!(error.kind, "query_cancelled");
    assert_eq!(error.data.unwrap()["sessionReset"], false);
    assert!(took < Duration::from_secs(5), "cancel took {took:?}");

    // Same session: the temp table is still there.
    let result = tab
        .run(&p, None, "SELECT v FROM kept", 10, None)
        .await
        .unwrap();
    assert_eq!(result.rows.len(), 1);
    tab.close().await;
}

#[tokio::test]
#[ignore]
async fn mysql_cancel_stops_the_query() {
    let tab = std::sync::Arc::new(QueryTab::default());
    let p = params(Engine::MySql);
    tab.run(&p, None, "SET @kept = 7", 10, None).await.unwrap();
    // An interrupted SLEEP() returns 1 instead of failing, so only the
    // timing shows it stopped.
    let (_, took) = run_and_cancel(Engine::MySql, &tab, "SELECT SLEEP(30)").await;
    assert!(took < Duration::from_secs(5), "cancel took {took:?}");
    let result = tab.run(&p, None, "SELECT @kept", 10, None).await.unwrap();
    assert_eq!(
        result.rows[0][0],
        thabi_studio_lib::database::Cell::Value("7".into())
    );
    tab.close().await;
}

#[tokio::test]
#[ignore]
async fn sqlserver_cancel_resets_the_session() {
    let tab = std::sync::Arc::new(QueryTab::default());
    let p = params(Engine::SqlServer);
    tab.run(&p, None, "SELECT 1", 10, None).await.unwrap();
    let (outcome, took) = run_and_cancel(Engine::SqlServer, &tab, "WAITFOR DELAY '00:00:30'").await;
    let error = outcome.unwrap_err();
    assert_eq!(error.kind, "query_cancelled");
    assert_eq!(error.data.unwrap()["sessionReset"], true);
    assert!(took < Duration::from_secs(5), "cancel took {took:?}");
    // A fresh session opens on the next run.
    assert_eq!(
        tab.run(&p, None, "SELECT 1", 10, None)
            .await
            .unwrap()
            .rows
            .len(),
        1
    );
    tab.close().await;
}

#[tokio::test]
#[ignore]
async fn timeouts_stop_long_queries() {
    let tab = QueryTab::default();
    let p = params(Engine::Postgres);
    let started = Instant::now();
    let error = tab
        .run(
            &p,
            None,
            "SELECT pg_sleep(30)",
            10,
            Some(Duration::from_secs(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.kind, "query_timed_out");
    assert!(started.elapsed() < Duration::from_secs(6));
    assert_eq!(
        tab.run(&p, None, "SELECT 1", 10, None)
            .await
            .unwrap()
            .rows
            .len(),
        1
    );
    tab.close().await;
}

#[tokio::test]
#[ignore]
async fn a_tab_runs_one_statement_at_a_time() {
    let tab = std::sync::Arc::new(QueryTab::default());
    let p = params(Engine::Postgres);
    let first = {
        let tab = tab.clone();
        let p = p.clone();
        tokio::spawn(async move { tab.run(&p, None, "SELECT pg_sleep(2)", 10, None).await })
    };
    tokio::time::sleep(Duration::from_millis(500)).await;
    let second = tab.run(&p, None, "SELECT 1", 10, None).await.unwrap_err();
    assert_eq!(second.kind, "query_running");
    first.await.unwrap().unwrap();
    tab.close().await;
}

#[tokio::test]
#[ignore]
async fn changing_database_opens_a_new_session() {
    let tab = QueryTab::default();
    let p = params(Engine::SqlServer);
    let result = tab
        .run(&p, Some("master".into()), "SELECT DB_NAME()", 10, None)
        .await
        .unwrap();
    assert_eq!(
        result.rows[0][0],
        thabi_studio_lib::database::Cell::Value("master".into())
    );
    let result = tab
        .run(&p, Some("tempdb".into()), "SELECT DB_NAME()", 10, None)
        .await
        .unwrap();
    assert_eq!(
        result.rows[0][0],
        thabi_studio_lib::database::Cell::Value("tempdb".into())
    );
    tab.close().await;
}

#[tokio::test]
#[ignore]
async fn large_results_stay_in_rust_and_export_to_csv() {
    use thabi_studio_lib::results::{Scope, Sort};
    let tab = QueryTab::default();
    let p = params(Engine::Postgres);
    let result = tab
        .run(
            &p,
            None,
            "SELECT n, 'row, ' || n AS label, CASE WHEN n % 10 = 0 THEN NULL ELSE n * 1.5 END AS amount \
             FROM generate_series(1, 5000) AS n",
            100_000,
            None,
        )
        .await
        .unwrap();
    assert_eq!(result.total_rows, 5000);
    assert_eq!(result.rows.len(), 200, "only the first page goes to the UI");
    let stored = tab.result(result.result_id.as_deref().unwrap()).unwrap();

    // Numeric sort, descending, NULLs last.
    let top = stored.page(
        0,
        3,
        Some(Sort {
            column: 2,
            descending: true,
        }),
    );
    assert_eq!(
        top[0][0],
        thabi_studio_lib::database::Cell::Value("4999".into())
    );
    let bottom = stored.page(
        4999,
        1,
        Some(Sort {
            column: 2,
            descending: true,
        }),
    );
    assert_eq!(bottom[0][2], thabi_studio_lib::database::Cell::Null);

    let path = std::env::temp_dir().join(format!("thabi-it-{}.csv", std::process::id()));
    let rows = stored
        .export_csv(
            &path,
            &Scope {
                include_header: true,
                ..Default::default()
            },
            None,
        )
        .unwrap();
    assert_eq!(rows, 5000);
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let mut lines = text.trim_start_matches('\u{feff}').split("\r\n");
    assert_eq!(lines.next(), Some("n,label,amount"));
    assert_eq!(lines.next(), Some("1,\"row, 1\",1.5"));
    assert!(
        text.contains("\r\n10,\"row, 10\",\r\n"),
        "NULL exports as an empty field"
    );
    tab.close().await;
}

#[tokio::test]
#[ignore]
async fn the_row_limit_is_reported() {
    let tab = QueryTab::default();
    let p = params(Engine::MySql);
    tab.run(
        &p,
        None,
        "SET SESSION cte_max_recursion_depth = 5000",
        10,
        None,
    )
    .await
    .unwrap();
    let result = tab
        .run(
            &p,
            None,
            "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 3000) SELECT i FROM n",
            1000,
            None,
        )
        .await
        .unwrap();
    assert_eq!(result.total_rows, 1000);
    assert!(result.truncated);
    tab.close().await;
}

/// Staged row edits run in one transaction; a change that finds no row
/// rolls back everything before it.
async fn applies_row_edits(engine: Engine, database: &str, schema: &str) {
    use thabi_studio_lib::database::{
        Cell,
        edit::{ColumnValue, RowChange, TableTarget, statements},
    };
    let tab = QueryTab::default();
    let p = params(engine);
    let name = "thabi_edit_it";
    let drop = match engine {
        Engine::SqlServer => format!("IF OBJECT_ID('{name}') IS NOT NULL DROP TABLE {name}"),
        _ => format!("DROP TABLE IF EXISTS {name}"),
    };
    tab.run(&p, None, &drop, 1, None).await.unwrap();
    tab.run(
        &p,
        None,
        &format!("CREATE TABLE {name} (id INT PRIMARY KEY, note VARCHAR(50) NULL, total DECIMAL(10, 2) NULL)"),
        1,
        None,
    )
    .await
    .unwrap();
    tab.run(
        &p,
        None,
        &format!("INSERT INTO {name} VALUES (1, 'a', 1.50), (2, 'b', 2.00)"),
        1,
        None,
    )
    .await
    .unwrap();

    let cv = |column: &str, value: Option<&str>| ColumnValue {
        column: column.into(),
        value: value.map(Into::into),
    };
    let target = TableTarget {
        database: database.into(),
        schema: schema.into(),
        name: name.into(),
    };
    let read = async |tab: &QueryTab| {
        let result = tab
            .run(
                &p,
                None,
                &format!("SELECT id, note, total FROM {name} ORDER BY id"),
                10,
                None,
            )
            .await
            .unwrap();
        result
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| match cell {
                        Cell::Value(v) => v.clone(),
                        _ => "NULL".into(),
                    })
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .collect::<Vec<_>>()
    };

    let changes = [
        RowChange::Update {
            key: vec![cv("id", Some("1"))],
            set: vec![cv("note", Some("O'Brien")), cv("total", None)],
        },
        RowChange::Delete {
            key: vec![cv("id", Some("2"))],
        },
        RowChange::Insert {
            values: vec![
                cv("id", Some("3")),
                cv("note", Some("x")),
                cv("total", Some("3.25")),
            ],
        },
    ];
    let sql = statements(engine, &target, &changes).unwrap();
    assert_eq!(tab.apply(&p, None, &sql).await.unwrap(), 3);
    assert_eq!(read(&tab).await, ["1|O'Brien|NULL", "3|x|3.25"]);

    // Deleting row 1 would succeed, but row 99 doesn't exist: nothing is saved.
    let conflict = [
        RowChange::Delete {
            key: vec![cv("id", Some("1"))],
        },
        RowChange::Update {
            key: vec![cv("id", Some("99"))],
            set: vec![cv("note", Some("z"))],
        },
    ];
    let sql = statements(engine, &target, &conflict).unwrap();
    let error = tab.apply(&p, None, &sql).await.unwrap_err();
    assert_eq!(error.kind, "edit_conflict", "{engine:?}: {}", error.message);
    assert_eq!(read(&tab).await, ["1|O'Brien|NULL", "3|x|3.25"]);

    tab.run(&p, None, &drop, 1, None).await.unwrap();
    tab.close().await;
}

#[tokio::test]
#[ignore]
async fn postgres_row_edits() {
    applies_row_edits(Engine::Postgres, "thabi_it", "public").await;
}

#[tokio::test]
#[ignore]
async fn mysql_row_edits() {
    applies_row_edits(Engine::MySql, "thabi_it", "").await;
}

#[tokio::test]
#[ignore]
async fn sqlserver_row_edits() {
    applies_row_edits(Engine::SqlServer, "master", "dbo").await;
}

/// Safe mode "read-only": on MySQL and PostgreSQL the server itself refuses
/// the write on a read-only session. SQL Server has no session equivalent, so
/// there the app check is the only guard (the UI says so) and the write is
/// expected to succeed — the test records that difference rather than hiding it.
async fn read_only_sessions_refuse_writes(engine: Engine) {
    let setup = QueryTab::default();
    let p = params(engine);
    let name = "thabi_readonly_it";
    let drop = match engine {
        Engine::SqlServer => format!("IF OBJECT_ID('{name}') IS NOT NULL DROP TABLE {name}"),
        _ => format!("DROP TABLE IF EXISTS {name}"),
    };
    setup.run(&p, None, &drop, 1, None).await.unwrap();
    setup
        .run(
            &p,
            None,
            &format!("CREATE TABLE {name} (id INT PRIMARY KEY)"),
            1,
            None,
        )
        .await
        .unwrap();
    setup
        .run(&p, None, &format!("INSERT INTO {name} VALUES (1)"), 1, None)
        .await
        .unwrap();

    let mut ro = params(engine);
    ro.read_only = true;
    let tab = QueryTab::default();
    let write = tab
        .run(
            &ro,
            None,
            &format!("INSERT INTO {name} VALUES (2)"),
            10,
            None,
        )
        .await;

    if engine == Engine::SqlServer {
        // No server-side guard here; the app blocks it before it is sent.
        write.expect("SQL Server cannot enforce read-only sessions");
    } else {
        let error = write.expect_err("a read-only session must refuse the write");
        assert_eq!(error.kind, "server", "{engine:?}: {}", error.message);
        // The server's own wording, kept in the detail for the user to read.
        let detail =
            format!("{}{}", error.message, error.detail.unwrap_or_default()).to_lowercase();
        assert!(
            detail.contains("read-only") || detail.contains("read only"),
            "{engine:?}: unexpected server error {detail:?}",
        );
    }

    // Reading still works on a read-only session. Only where the server
    // enforced the mode is the table left unchanged.
    let read = tab
        .run(&ro, None, &format!("SELECT id FROM {name}"), 10, None)
        .await
        .unwrap();
    assert_eq!(
        read.rows.len(),
        if engine == Engine::SqlServer { 2 } else { 1 },
        "{engine:?}"
    );

    setup.run(&p, None, &drop, 1, None).await.unwrap();
    tab.close().await;
    setup.close().await;
}

#[tokio::test]
#[ignore]
async fn postgres_read_only_session() {
    read_only_sessions_refuse_writes(Engine::Postgres).await;
}

#[tokio::test]
#[ignore]
async fn mysql_read_only_session() {
    read_only_sessions_refuse_writes(Engine::MySql).await;
}

#[tokio::test]
#[ignore]
async fn sqlserver_has_no_read_only_session() {
    read_only_sessions_refuse_writes(Engine::SqlServer).await;
}

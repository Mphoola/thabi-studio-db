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
        client_cert: None,
        client_key: None,
        ssh: None,
        connect_to: None,
        read_only: false,
        startup_commands: Vec::new(),
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
        use_default: false,
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

/// Startup commands run on every new session, including the session a query
/// tab opens for itself — not just the connection's own.
#[tokio::test]
#[ignore]
async fn startup_commands_apply_to_query_tab_sessions() {
    let tab = QueryTab::default();
    let mut p = params(Engine::Postgres);
    p.startup_commands = vec!["SET search_path = pg_catalog".into()];
    let result = tab
        .run(&p, None, "SELECT current_schema() AS schema", 10, None)
        .await
        .unwrap();
    let schema = match &result.rows[0][0] {
        thabi_studio_lib::database::Cell::Value(value) => value.clone(),
        other => panic!("expected a schema name, got {other:?}"),
    };
    assert_eq!(schema, "pg_catalog", "the tab's session kept the setting");
    tab.close().await;
}

/// A startup command that fails stops the connection, with the statement named.
#[tokio::test]
#[ignore]
async fn a_failing_startup_command_stops_the_connect() {
    let tab = QueryTab::default();
    let mut p = params(Engine::Postgres);
    p.startup_commands = vec!["SET nonexistent_setting = 1".into()];
    let error = tab.run(&p, None, "SELECT 1", 10, None).await.unwrap_err();
    assert_eq!(error.kind, "startup_command_failed");
    let detail = error.detail.unwrap_or_default();
    assert!(detail.contains("nonexistent_setting"), "{detail}");
    tab.close().await;
}

/// Phase 12: a table is read a page at a time from the server, ordered by its
/// primary key, and sorting a page re-queries rather than reordering what
/// happens to be loaded. 10,000 rows over a composite key, so a page boundary
/// that loses or repeats a row shows up.
async fn pages_through_a_table(engine: Engine, database: &str, schema: &str) {
    use thabi_studio_lib::database::paging::{PageOrder, count_sql, page_sql};
    const ROWS: i64 = 10_000;
    let tab = std::sync::Arc::new(QueryTab::default());
    let p = params(engine);
    let name = "thabi_page_it";
    let qualified = match engine {
        Engine::Postgres => format!("\"{schema}\".\"{name}\""),
        Engine::MySql => format!("`{database}`.`{name}`"),
        Engine::SqlServer => format!("[{database}].[{schema}].[{name}]"),
    };
    let drop = match engine {
        Engine::SqlServer => format!("IF OBJECT_ID('{name}') IS NOT NULL DROP TABLE {name}"),
        _ => format!("DROP TABLE IF EXISTS {name}"),
    };
    tab.run(&p, None, &drop, 1, None).await.unwrap();
    tab.run(
        &p,
        None,
        &format!("CREATE TABLE {name} (tenant_id INT NOT NULL, id INT NOT NULL, value_txt VARCHAR(40) NULL, PRIMARY KEY (tenant_id, id))"),
        1,
        None,
    )
    .await
    .unwrap();

    // Two tenants, so a page boundary lands in the middle of one of them.
    for tenant in 0..2 {
        for id in 0..(ROWS / 2) {
            tab.run(
                &p,
                None,
                &format!("INSERT INTO {name} VALUES ({tenant}, {id}, 'row-{tenant}-{id}')"),
                1,
                None,
            )
            .await
            .unwrap();
        }
    }

    let key = vec!["tenant_id".to_owned(), "id".to_owned()];
    let page = |offset: usize, limit: usize, order: Option<PageOrder>| {
        let sql = page_sql(
            engine,
            database,
            schema,
            name,
            "",
            order.as_ref(),
            &key,
            offset,
            limit,
        );
        let tab = tab.clone();
        let p = p.clone();
        async move {
            let result = tab.run(&p, None, &sql, limit, None).await.unwrap();
            result
                .rows
                .into_iter()
                .map(|row| match &row[2] {
                    thabi_studio_lib::database::Cell::Value(value) => value.clone(),
                    _ => String::new(),
                })
                .collect::<Vec<String>>()
        }
    };

    // Walk the whole table a page at a time: every row exactly once, in key order.
    let size = 200;
    let mut seen: Vec<String> = Vec::new();
    let mut offset = 0;
    loop {
        let rows = page(offset, size, None).await;
        if rows.is_empty() {
            break;
        }
        assert!(
            rows.len() <= size,
            "a page never returns more than it asked for"
        );
        seen.extend(rows);
        offset += size;
        if offset > ROWS as usize + size {
            break;
        }
    }
    assert_eq!(seen.len(), ROWS as usize, "every row is reached once");
    // Key order is tenant, then id: comparing against that exact sequence
    // proves the pages neither lost, repeated nor reordered a row.
    let expected: Vec<String> = (0..2)
        .flat_map(|tenant| (0..(ROWS / 2)).map(move |id| format!("row-{tenant}-{id}")))
        .collect();
    assert_eq!(seen, expected, "rows come back once each, in key order");

    // Sorting by a column re-queries: the first page of a descending sort is
    // the largest value, which sorting the loaded rows could never produce.
    // Sorted as text, so the largest value is not the largest id.
    let largest = expected.iter().max().cloned().expect("rows exist");
    let sorted = page(
        0,
        10,
        Some(PageOrder {
            column: "value_txt".into(),
            descending: true,
        }),
    )
    .await;
    assert_eq!(
        sorted.first().cloned(),
        Some(largest),
        "sorted on the server"
    );

    // The exact count is available when the estimate isn't trusted.
    let count_sql = count_sql(engine, database, schema, name, "");
    let result = tab.run(&p, None, &count_sql, 1, None).await.unwrap();
    let count = match &result.rows[0][0] {
        thabi_studio_lib::database::Cell::Value(value) => value.parse::<i64>().unwrap(),
        _ => panic!("expected a count"),
    };
    assert_eq!(count, ROWS, "{engine:?}: exact count over {qualified}");

    tab.run(&p, None, &drop, 1, None).await.unwrap();
    tab.close().await;
}

#[tokio::test]
#[ignore]
async fn postgres_pages_a_large_table() {
    pages_through_a_table(Engine::Postgres, "thabi_it", "public").await;
}

#[tokio::test]
#[ignore]
async fn mysql_pages_a_large_table() {
    pages_through_a_table(Engine::MySql, "thabi_it", "").await;
}

#[tokio::test]
#[ignore]
async fn sqlserver_pages_a_large_table() {
    pages_through_a_table(Engine::SqlServer, "master", "dbo").await;
}

/// Phase 12: copy and export re-read the table from the server, so a whole
/// table is covered rather than one page, and the text round-trips Unicode,
/// NULLs, quotes and a big number.
async fn streams_a_table_out(engine: Engine, database: &str, schema: &str) {
    use thabi_studio_lib::database::transfer::{TransferFormat, TransferRequest, stream_table};
    let tab = std::sync::Arc::new(QueryTab::default());
    let p = params(engine);
    let name = "thabi_copy_it";
    let drop = match engine {
        Engine::SqlServer => format!("IF OBJECT_ID('{name}') IS NOT NULL DROP TABLE {name}"),
        _ => format!("DROP TABLE IF EXISTS {name}"),
    };
    tab.run(&p, None, &drop, 1, None).await.unwrap();
    tab.run(
        &p,
        None,
        // NVARCHAR on SQL Server, and the sample text stays inside Latin-1:
        // the test container's default collation cannot store U+2713, and a
        // character the server cannot hold would fail here rather than in the
        // app. The app itself round-trips whatever the server returns.
        &format!(
            "CREATE TABLE {name} (id INT PRIMARY KEY, label {} NULL)",
            if engine == Engine::SqlServer {
                "NVARCHAR(80)"
            } else {
                "VARCHAR(80)"
            }
        ),
        1,
        None,
    )
    .await
    .unwrap();
    tab.run(
        &p,
        None,
        &format!(
            "INSERT INTO {name} VALUES (1, 'plain'), (2, 'Zoë ünïcode'), (3, 'O''Brien'), (4, NULL), (5, '12345678901234567890'), (6, 'a,b \"c\"')"
        ),
        1,
        None,
    )
    .await
    .unwrap();

    let request = |rows: Option<(usize, usize)>, header: bool| TransferRequest {
        database: database.into(),
        schema: schema.into(),
        name: name.into(),
        primary_key: vec!["id".into()],
        columns: vec!["id".into(), "label".into()],
        rows,
        include_header: header,
        ..TransferRequest::default()
    };

    // Everything, not just the first page.
    let mut csv: Vec<u8> = Vec::new();
    let written = stream_table(
        &tab,
        &p,
        &request(None, true),
        TransferFormat::Csv,
        None,
        &mut csv,
    )
    .await
    .unwrap();
    assert_eq!(written.rows, 6, "{engine:?}: every row, not one page");
    let csv = String::from_utf8(csv).unwrap();
    assert!(csv.starts_with("id,label\r\n"), "{csv}");
    assert!(csv.contains("Zoë ünïcode"), "Unicode survives: {csv}");
    assert!(
        csv.contains("3,O'Brien\r\n"),
        "an apostrophe needs no quotes: {csv}"
    );
    assert!(
        csv.contains("\"a,b \"\"c\"\"\""),
        "a comma and quotes are escaped RFC 4180: {csv}"
    );
    assert!(
        csv.contains("12345678901234567890"),
        "big numbers stay text: {csv}"
    );
    assert!(
        csv.contains("1,plain\r\n") && csv.contains("4,\r\n"),
        "{csv}"
    );

    // A scope: only some rows, without the header.
    let mut slice: Vec<u8> = Vec::new();
    let written = stream_table(
        &tab,
        &p,
        &request(Some((1, 3)), false),
        TransferFormat::Csv,
        None,
        &mut slice,
    )
    .await
    .unwrap();
    assert_eq!(written.rows, 2);
    let slice = String::from_utf8(slice).unwrap();
    assert!(!slice.contains("id,label"), "no header when not asked for");
    assert!(
        slice.contains("Zoë ünïcode") && !slice.contains("plain"),
        "{slice}"
    );

    // The SQL statements are the ones a user would actually run.
    let mut insert: Vec<u8> = Vec::new();
    stream_table(
        &tab,
        &p,
        &request(None, false),
        TransferFormat::SqlInsert,
        None,
        &mut insert,
    )
    .await
    .unwrap();
    let insert = String::from_utf8(insert).unwrap();
    assert_eq!(
        insert.lines().filter(|l| l.starts_with("INSERT")).count(),
        6
    );

    // Active filters apply to copy and export, not just to the grid.
    let mut filtered: Vec<u8> = Vec::new();
    let mut with_filter = request(None, false);
    with_filter.filters = vec![thabi_studio_lib::database::filter::RowFilter {
        column: "id".into(),
        op: "le".into(),
        value: "2".into(),
    }];
    let written = stream_table(
        &tab,
        &p,
        &with_filter,
        TransferFormat::Csv,
        None,
        &mut filtered,
    )
    .await
    .unwrap();
    assert_eq!(written.rows, 2, "{engine:?}: the filter is honoured");

    // JSON is a real document, keyed by column.
    let mut json: Vec<u8> = Vec::new();
    stream_table(
        &tab,
        &p,
        &request(None, false),
        TransferFormat::Json,
        None,
        &mut json,
    )
    .await
    .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&json).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 6);
    assert_eq!(json[1]["label"], "Zoë ünïcode");
    assert!(json[3]["label"].is_null());

    // The INSERT script really runs: empty the table and replay it.
    tab.run(&p, None, &format!("DELETE FROM {name}"), 1, None)
        .await
        .unwrap();
    for statement in insert.lines().filter(|l| l.starts_with("INSERT")) {
        tab.run(&p, None, statement.trim_end_matches(';'), 1, None)
            .await
            .unwrap_or_else(|e| panic!("{engine:?}: {statement}: {}", e.message));
    }
    let mut again: Vec<u8> = Vec::new();
    stream_table(
        &tab,
        &p,
        &request(None, true),
        TransferFormat::Csv,
        None,
        &mut again,
    )
    .await
    .unwrap();
    assert_eq!(
        String::from_utf8(again).unwrap(),
        csv,
        "{engine:?}: INSERT round-trip"
    );

    // Binary longer than the grid's preview is exported in full and replays.
    let blob = "thabi_blob_it";
    let drop_blob = match engine {
        Engine::SqlServer => format!("IF OBJECT_ID('{blob}') IS NOT NULL DROP TABLE {blob}"),
        _ => format!("DROP TABLE IF EXISTS {blob}"),
    };
    let (blob_type, value) = {
        let hex: String = (0u8..40).map(|b| format!("{b:02X}")).collect();
        match engine {
            Engine::Postgres => ("BYTEA", format!("'\\x{hex}'::bytea")),
            Engine::MySql => ("VARBINARY(64)", format!("X'{hex}'")),
            Engine::SqlServer => ("VARBINARY(64)", format!("0x{hex}")),
        }
    };
    tab.run(&p, None, &drop_blob, 1, None).await.unwrap();
    tab.run(
        &p,
        None,
        &format!("CREATE TABLE {blob} (id INT PRIMARY KEY, data {blob_type})"),
        1,
        None,
    )
    .await
    .unwrap();
    tab.run(
        &p,
        None,
        &format!("INSERT INTO {blob} VALUES (1, {value})"),
        1,
        None,
    )
    .await
    .unwrap();
    let blob_request = TransferRequest {
        database: database.into(),
        schema: schema.into(),
        name: blob.into(),
        primary_key: vec!["id".into()],
        columns: vec!["id".into(), "data".into()],
        ..TransferRequest::default()
    };
    let read = async || {
        let mut out: Vec<u8> = Vec::new();
        stream_table(&tab, &p, &blob_request, TransferFormat::Csv, None, &mut out)
            .await
            .unwrap();
        String::from_utf8(out).unwrap()
    };
    let before = read().await;
    let mut script: Vec<u8> = Vec::new();
    stream_table(
        &tab,
        &p,
        &blob_request,
        TransferFormat::SqlInsert,
        None,
        &mut script,
    )
    .await
    .unwrap();
    let script = String::from_utf8(script).unwrap();
    tab.run(&p, None, &format!("DELETE FROM {blob}"), 1, None)
        .await
        .unwrap();
    tab.run(&p, None, script.trim().trim_end_matches(';'), 1, None)
        .await
        .unwrap_or_else(|e| panic!("{engine:?}: {script}: {}", e.message));
    assert_eq!(read().await, before, "{engine:?}: binary round-trip");
    assert!(
        before.contains("2627"),
        "all 40 bytes, not a preview: {before}"
    );
    tab.run(&p, None, &drop_blob, 1, None).await.unwrap();

    // Copy is capped, so a huge table cannot become a gigabyte of clipboard.
    let mut capped: Vec<u8> = Vec::new();
    let written = stream_table(
        &tab,
        &p,
        &request(None, false),
        TransferFormat::Csv,
        Some(2),
        &mut capped,
    )
    .await
    .unwrap();
    assert_eq!(written.rows, 2, "the cap is honoured");

    tab.run(&p, None, &drop, 1, None).await.unwrap();
    tab.close().await;
}

#[tokio::test]
#[ignore]
async fn postgres_streams_a_table_out() {
    streams_a_table_out(Engine::Postgres, "thabi_it", "public").await;
}

#[tokio::test]
#[ignore]
async fn mysql_streams_a_table_out() {
    streams_a_table_out(Engine::MySql, "thabi_it", "").await;
}

#[tokio::test]
#[ignore]
async fn sqlserver_streams_a_table_out() {
    streams_a_table_out(Engine::SqlServer, "master", "dbo").await;
}

/// A batch keeps every row set; each is its own result.
async fn keeps_every_row_set(engine: Engine, batch: &str) {
    let tab = QueryTab::default();
    let p = params(engine);
    let result = tab.run(&p, None, batch, 100, None).await.unwrap();
    assert_eq!(result.columns[0].name, "a", "{engine:?}");
    assert_eq!(result.more.len(), 1, "{engine:?}: {result:?}");
    assert_eq!(result.more[0].columns.len(), 2);
    assert!(
        result.more[0].result_id.is_some(),
        "kept for paging and export"
    );
    tab.close().await;
}

#[tokio::test]
#[ignore]
async fn postgres_keeps_every_row_set() {
    keeps_every_row_set(Engine::Postgres, "SELECT 1 AS a; SELECT 2 AS b, 3 AS c").await;
}

#[tokio::test]
#[ignore]
async fn mysql_keeps_every_row_set() {
    keeps_every_row_set(Engine::MySql, "SELECT 1 AS a; SELECT 2 AS b, 3 AS c").await;
}

#[tokio::test]
#[ignore]
async fn sqlserver_keeps_every_row_set() {
    keeps_every_row_set(Engine::SqlServer, "SELECT 1 AS a; SELECT 2 AS b, 3 AS c").await;
}

/// Rows that arrived before a cancel are kept and handed back, and progress
/// is visible while the statement runs.
#[tokio::test]
#[ignore]
async fn postgres_cancel_keeps_rows_that_arrived() {
    let tab = std::sync::Arc::new(QueryTab::default());
    let p = params(Engine::Postgres);
    let runner = {
        let tab = tab.clone();
        let p = p.clone();
        tokio::spawn(async move {
            tab.run(
                &p,
                None,
                // Enough rows to fill the server's send buffer, so they are
                // flushed before it reaches the slow last row.
                "SELECT g, CASE WHEN g = 200000 THEN pg_sleep(30)::text END AS s FROM generate_series(1, 200000) g",
                100,
                None,
            )
            .await
        })
    };
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let progress = tab.progress().expect("running");
    assert!(
        progress.received > 1000,
        "rows arrive while it runs: {}",
        progress.received
    );
    assert_eq!(
        progress.preview.unwrap().rows.len(),
        100,
        "capped at the row limit"
    );
    assert!(tab.cancel(&p));
    let error = runner.await.unwrap().unwrap_err();
    assert_eq!(error.kind, "query_cancelled");
    let partial = &error.data.as_ref().unwrap()["partial"];
    assert_eq!(
        partial["totalRows"], 100,
        "the rows that arrived, up to the limit"
    );
    assert_eq!(partial["truncated"], true);
    tab.close().await;
}

/// A SQL file runs statement by statement, engine syntax included, and
/// stops at the first error naming its line.
async fn runs_a_sql_file(engine: Engine, script: &str, failing_line: usize) {
    use thabi_studio_lib::database::script::split_script;
    let tab = QueryTab::default();
    let p = params(engine);
    let statements = split_script(script, engine);
    let summary = tab.run_script(&p, None, &statements, false).await.unwrap();
    assert_eq!(summary.errors.len(), 1, "{engine:?}: {summary:?}");
    assert_eq!(summary.errors[0].line, failing_line, "{engine:?}");
    assert_eq!(summary.ran, statements.len() - 2, "stops at the error");
    let count = tab
        .run(&p, None, "SELECT COUNT(*) FROM thabi_script", 1, None)
        .await
        .unwrap();
    assert_eq!(
        count.rows[0][0],
        thabi_studio_lib::database::Cell::Value("2".into())
    );
    tab.close().await;
}

#[tokio::test]
#[ignore]
async fn postgres_runs_a_sql_file() {
    runs_a_sql_file(
        Engine::Postgres,
        "DROP TABLE IF EXISTS thabi_script;\nCREATE TABLE thabi_script (id int);\nCREATE OR REPLACE FUNCTION thabi_add(n int) RETURNS void AS $$\nBEGIN INSERT INTO thabi_script VALUES (n); END;\n$$ LANGUAGE plpgsql;\nSELECT thabi_add(1);\nSELECT thabi_add(2);\nSELECT * FROM no_such_table_xyz;\nSELECT thabi_add(3);",
        8,
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn mysql_runs_a_sql_file() {
    runs_a_sql_file(
        Engine::MySql,
        "DROP TABLE IF EXISTS thabi_script;\nCREATE TABLE thabi_script (id int);\nDROP PROCEDURE IF EXISTS thabi_add;\nDELIMITER //\nCREATE PROCEDURE thabi_add(n int) BEGIN INSERT INTO thabi_script VALUES (n); SELECT 1; END//\nDELIMITER ;\nCALL thabi_add(1);\nCALL thabi_add(2);\nSELECT * FROM no_such_table_xyz;\nCALL thabi_add(3);",
        9,
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn sqlserver_runs_a_sql_file() {
    runs_a_sql_file(
        Engine::SqlServer,
        "IF OBJECT_ID('thabi_script') IS NOT NULL DROP TABLE thabi_script\nGO\nCREATE TABLE thabi_script (id int)\nGO\nINSERT INTO thabi_script VALUES (1);\nINSERT INTO thabi_script VALUES (2)\nGO\nSELECT * FROM no_such_table_xyz\nGO\nINSERT INTO thabi_script VALUES (3)",
        8,
    )
    .await;
}

/// Editing a query result: the key comes from the rows the query returned,
/// exactly as the grid hands it to the editor, and the change lands on the
/// row that was shown.
async fn edits_a_query_result(engine: Engine, database: &str, schema: &str) {
    use thabi_studio_lib::database::{
        Cell,
        edit::{ColumnValue, RowChange, TableTarget, statements},
    };
    let tab = QueryTab::default();
    let p = params(engine);
    let name = "thabi_result_edit";
    let drop = match engine {
        Engine::SqlServer => format!("IF OBJECT_ID('{name}') IS NOT NULL DROP TABLE {name}"),
        _ => format!("DROP TABLE IF EXISTS {name}"),
    };
    for sql in [
        drop.clone(),
        format!("CREATE TABLE {name} (id INT PRIMARY KEY, note VARCHAR(20))"),
        format!("INSERT INTO {name} VALUES (1, 'a'), (2, 'b'), (3, 'c')"),
    ] {
        tab.run(&p, None, &sql, 1, None).await.unwrap();
    }
    // The user's query, filtered and reordered: row 0 of the result is id 3.
    let result = tab
        .run(
            &p,
            None,
            &format!("SELECT note, id FROM {name} WHERE id > 1 ORDER BY id DESC"),
            10,
            None,
        )
        .await
        .unwrap();
    let Cell::Value(key) = &result.rows[0][1] else {
        panic!("id is a value");
    };
    let change = RowChange::Update {
        key: vec![ColumnValue {
            column: "id".into(),
            value: Some(key.clone()),
            use_default: false,
        }],
        set: vec![ColumnValue {
            column: "note".into(),
            value: Some("edited".into()),
            use_default: false,
        }],
    };
    let target = TableTarget {
        database: database.into(),
        schema: schema.into(),
        name: name.into(),
    };
    let sql = statements(engine, &target, &[change]).unwrap();
    assert_eq!(tab.apply(&p, None, &sql).await.unwrap(), 1);
    let after = tab
        .run(
            &p,
            None,
            &format!("SELECT note FROM {name} ORDER BY id"),
            10,
            None,
        )
        .await
        .unwrap();
    let notes: Vec<_> = after.rows.iter().map(|r| r[0].clone()).collect();
    assert_eq!(
        notes,
        [
            Cell::Value("a".into()),
            Cell::Value("b".into()),
            Cell::Value("edited".into())
        ],
        "{engine:?}"
    );
    tab.run(&p, None, &drop, 1, None).await.unwrap();
    tab.close().await;
}

#[tokio::test]
#[ignore]
async fn postgres_edits_a_query_result() {
    edits_a_query_result(Engine::Postgres, "thabi_it", "public").await;
}

#[tokio::test]
#[ignore]
async fn mysql_edits_a_query_result() {
    edits_a_query_result(Engine::MySql, "thabi_it", "").await;
}

#[tokio::test]
#[ignore]
async fn sqlserver_edits_a_query_result() {
    edits_a_query_result(Engine::SqlServer, "master", "dbo").await;
}

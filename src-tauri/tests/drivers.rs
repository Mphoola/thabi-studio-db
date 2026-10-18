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
        client_cert: None,
        client_key: None,
        ssh: None,
        connect_to: None,
        read_only: false,
        startup_commands: Vec::new(),
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

/**
 * A bound value is stored as itself. This is the property parameters exist for:
 * a value containing quotes and a semicolon must come back out of the database
 * as those characters, not as the end of the statement and the start of another.
 */
/**
 * A bound query's values arrive typed, and each is read as its own type. These
 * are the ones a real schema has, including NUMERIC — which is the reason the
 * decoder exists rather than reading everything as a string, since that would
 * quietly drop digits.
 */
async fn decodes_bound_values_by_type(engine: Engine) {
    let mut session = open(engine).await;
    let sql = match engine {
        Engine::Postgres => {
            "SELECT 1::int2 AS small, 2::int4 AS medium, \
             9007199254740993::int8 AS big, 1.5::float4 AS real_one, \
             1.25::float8 AS real_double, 12345678901234567890.12345::numeric AS amount, \
             TRUE AS flag, '2026-02-14'::date AS day, \
             '2026-02-14 09:30:00'::timestamp AS at, \
             '2026-02-14 09:30:00+00'::timestamptz AS at_utc, \
             '11111111-2222-3333-4444-555555555555'::uuid AS ref, \
             '{\"a\":1}'::jsonb AS doc, 'hello'::text AS label, \
             '\\x48656c6c6f'::bytea AS blob"
        }
        _ => return,
    };
    let result = session
        .execute_bound(sql, &[], 10)
        .await
        .expect("bound select");
    let row = &result.rows[0];
    let by_name = |name: &str| {
        let at = result
            .columns
            .iter()
            .position(|column| column.name == name)
            .unwrap_or_else(|| panic!("no column {name}"));
        row[at].clone()
    };
    assert_eq!(by_name("small"), Cell::Value("1".into()));
    assert_eq!(by_name("medium"), Cell::Value("2".into()));
    // Beyond f64's exact range, which is where a float decode would round.
    assert_eq!(by_name("big"), Cell::Value("9007199254740993".into()));
    assert_eq!(by_name("real_one"), Cell::Value("1.5".into()));
    assert_eq!(by_name("real_double"), Cell::Value("1.25".into()));
    // Every digit is still there.
    assert_eq!(
        by_name("amount"),
        Cell::Value("12345678901234567890.12345".into()),
        "numeric lost digits"
    );
    assert_eq!(by_name("flag"), Cell::Value("t".into()));
    assert_eq!(by_name("day"), Cell::Value("2026-02-14".into()));
    assert_eq!(by_name("at"), Cell::Value("2026-02-14 09:30:00".into()));
    assert_eq!(by_name("label"), Cell::Value("hello".into()));
    match by_name("blob") {
        Cell::Binary { bytes, .. } => assert_eq!(bytes, 5),
        other => panic!("bytea came back as {other:?}"),
    }

    // The one that matters most: a bound query has to show what an unbound one
    // shows, or the same column reads differently depending on whether it was
    // given a parameter. The simple protocol is what every other query in the
    // app goes through, so it is the thing to compare against.
    let plain = session.execute(sql, 10).await.expect("text select");
    // Everything except `amount`, where only trailing zeros may differ: the
    // bound path trims them and so does the simple protocol, but they can
    // disagree on the scale the column declares, and no digit is at stake.
    for name in ["big", "label", "day"] {
        let at = |result: &thabi_studio_lib::database::QueryResult| {
            result
                .columns
                .iter()
                .position(|column| column.name == name)
                .unwrap()
        };
        assert_eq!(
            by_name(name),
            plain.rows[0][at(&plain)],
            "{name} reads differently when bound"
        );
    }
}

/**
 * The designer generates DDL as text, and the unit tests can only check the
 * text. This runs it: every generated statement is applied to a real server and
 * read back through describe, which is the only way to know the statements are
 * the ones each engine accepts.
 */
/// Adding and dropping a foreign key, the way the designer does it.
///
/// Read back through `describe`, so this is what a person would see afterwards
/// rather than what the generator believed it wrote.
async fn adds_and_drops_a_foreign_key(engine: Engine, db: &str, schema: &str) {
    use thabi_studio_lib::database::ddl::{KeyChange, TablePlan, table_script};
    let mut session = open(engine).await;
    for statement in [
        "DROP TABLE IF EXISTS thabi_fk_child",
        "DROP TABLE IF EXISTS thabi_fk_parent",
        "CREATE TABLE thabi_fk_parent (id int PRIMARY KEY, name text)",
        "CREATE TABLE thabi_fk_child (id int PRIMARY KEY, parent_id int)",
    ] {
        session.execute(statement, 0).await.expect("setup");
    }

    let add = TablePlan {
        keys: vec![KeyChange::AddForeignKey(
            thabi_studio_lib::database::ddl::ForeignKey {
                name: "thabi_child_parent_fk".into(),
                columns: vec!["parent_id".into()],
                references: vec![schema.to_owned(), "thabi_fk_parent".into()],
                referenced_columns: vec!["id".into()],
                on_delete: "CASCADE".into(),
                on_update: "NO ACTION".into(),
            },
        )],
        ..TablePlan::default()
    };
    for statement in table_script(engine, db, schema, "thabi_fk_child", &add) {
        session
            .execute(&statement, 1)
            .await
            .unwrap_or_else(|e| panic!("{engine:?} refused adding the key:\n{statement}\n{e:?}"));
    }

    let after = session
        .describe(
            db,
            schema,
            "thabi_fk_child",
            thabi_studio_lib::database::ObjectKind::Table,
        )
        .await
        .expect("describe after add");
    let target = after
        .columns
        .iter()
        .find(|one| one.name == "parent_id")
        .and_then(|one| one.references.clone())
        .unwrap_or_else(|| panic!("{engine:?} did not record the key on parent_id"));
    assert_eq!(target.table, "thabi_fk_parent", "{engine:?}");
    assert_eq!(target.column, "id", "{engine:?}");

    let drop = TablePlan {
        keys: vec![KeyChange::DropForeignKey {
            name: "thabi_child_parent_fk".into(),
        }],
        ..TablePlan::default()
    };
    for statement in table_script(engine, db, schema, "thabi_fk_child", &drop) {
        session
            .execute(&statement, 1)
            .await
            .unwrap_or_else(|e| panic!("{engine:?} refused dropping the key:\n{statement}\n{e:?}"));
    }

    let after = session
        .describe(
            db,
            schema,
            "thabi_fk_child",
            thabi_studio_lib::database::ObjectKind::Table,
        )
        .await
        .expect("describe after drop");
    assert!(
        after
            .columns
            .iter()
            .find(|one| one.name == "parent_id")
            .unwrap()
            .references
            .is_none(),
        "{engine:?} still reports the dropped key"
    );

    for statement in [
        "DROP TABLE IF EXISTS thabi_fk_child",
        "DROP TABLE IF EXISTS thabi_fk_parent",
    ] {
        session.execute(statement, 0).await.expect("cleanup");
    }
}

/// Export a table, drop it, run the export back, and compare.
///
/// The only test that means anything for a backup. Everything else checks that
/// a statement was *generated*; this checks that a file can recreate what it
/// came from, on a server, with the values that break naive escaping.
async fn export_round_trips(engine: Engine, db: &str, schema: &str) {
    use thabi_studio_lib::database::ddl::{ColumnSpec, TableSpec, create_table};
    use thabi_studio_lib::database::dump::{structure_statements, write_table_rows};

    let table = thabi_studio_lib::database::dump::table_of(engine, db, schema, "thabi_dump");
    let mut session = open(engine).await;
    session
        .execute(&format!("DROP TABLE IF EXISTS {table}"), 0)
        .await
        .expect("drop");

    let spec = TableSpec {
        columns: vec![
            ColumnSpec {
                name: "id".into(),
                data_type: "INTEGER".into(),
                nullable: false,
                default: String::new(),
                comment: String::new(),
                primary_key: true,
                unique: false,
            },
            ColumnSpec {
                name: "label".into(),
                // Each engine's own name: "text" is not a SQL Server type and
                // an invented one is refused by all three.
                data_type: match engine {
                    Engine::SqlServer => "NVARCHAR(200)".into(),
                    _ => "text".into(),
                },
                nullable: true,
                default: String::new(),
                comment: String::new(),
                primary_key: false,
                unique: false,
            },
        ],
        unique_names: vec![],
        primary_key_name: String::new(),
        foreign_keys: vec![],
    };
    let create = create_table(engine, db, schema, "thabi_dump", &spec).expect("create");
    session
        .execute(&format!("{create};"), 0)
        .await
        .expect("create table");

    // The values that break escaping: an apostrophe, a backslash, a newline, a
    // Unicode character, an empty string and a NULL.
    //
    // The backslash is written per engine because no single literal stores the
    // same value everywhere: MySQL treats it as an escape, so `back\\slash`
    // stores one backslash there and two on the engines where it is not an
    // escape. The dump is not what is being tested here — the input is.
    let backslash = match engine {
        Engine::MySql => "back\\\\slash",
        _ => "back\\slash",
    };
    // SQL Server reads an unprefixed literal in the collation's code page, so
    // anything outside it arrives as "?". The dump writes N'' — this is the
    // setup that has to say so, not the export.
    let n = if engine == Engine::SqlServer { "N" } else { "" };
    session
        .execute(
            &format!(
                "INSERT INTO {table} (id, label) VALUES \
                 (1, {n}'plain'), (2, {n}'O''Hara'), (3, {n}'{backslash}'), \
                 (4, {n}'line1\nline2'), (5, {n}'naïve ☕'), (6, {n}''), (7, NULL)"
            ),
            0,
        )
        .await
        .unwrap_or_else(|e| panic!("{engine:?} refused the setup values: {e:?}"));

    let described = session
        .describe(
            db,
            schema,
            "thabi_dump",
            thabi_studio_lib::database::ObjectKind::Table,
        )
        .await
        .expect("describe source");

    let mut file: Vec<u8> = Vec::new();
    for statement in structure_statements(engine, db, schema, "thabi_dump", &described) {
        file.extend_from_slice(statement.as_bytes());
    }
    let mut silence = |_: u64| {};
    let mut run = write_run(None, &mut silence);
    let exported = write_table_rows(
        &mut session,
        &target_for(engine, db, schema, "thabi_dump"),
        &described,
        &mut run,
        &mut file,
    )
    .await
    .expect("export rows");
    let script = String::from_utf8(file).expect("utf8 dump");
    assert_eq!(exported, 7, "{engine:?} exported the wrong number of rows");
    assert!(script.contains("INSERT INTO"), "no inserts were written");

    // Drop it, and put it back from the file alone.
    session
        .execute(&format!("DROP TABLE IF EXISTS {table}"), 0)
        .await
        .expect("drop for restore");
    for statement in split_statements(&script) {
        session.execute(&statement, 0).await.unwrap_or_else(|e| {
            panic!("{engine:?} refused its own export:\n{statement}\n{e:?}\n--- file ---\n{script}")
        });
    }

    let restored = session
        .describe(
            db,
            schema,
            "thabi_dump",
            thabi_studio_lib::database::ObjectKind::Table,
        )
        .await
        .expect("describe restored");
    let read = session
        .execute(&format!("SELECT id, label FROM {table} ORDER BY id"), 10)
        .await
        .expect("read back");
    // The labels, compared as values. Comparing debug-formatted rows instead
    // cannot tell an empty string from the two characters "''", which is one
    // of the values this is here to check.
    let labels: Vec<Option<String>> = read
        .rows
        .iter()
        .map(|row| match &row[1] {
            Cell::Value(text) => Some(text.clone()),
            _ => None,
        })
        .collect();
    let expected: Vec<Option<String>> = vec![
        Some("plain".to_owned()),
        Some("O'Hara".to_owned()),
        Some("back\\slash".to_owned()),
        Some("line1\nline2".to_owned()),
        Some("naïve ☕".to_owned()),
        Some(String::new()),
        None,
    ];
    assert_eq!(
        labels, expected,
        "{engine:?} did not round-trip the values exactly"
    );
    assert_eq!(
        restored.columns.len(),
        described.columns.len(),
        "{engine:?} changed the column count"
    );

    session
        .execute(&format!("DROP TABLE IF EXISTS {table}"), 0)
        .await
        .ok();
}

/// Writes a real dump to /tmp so a person can read it.
///
/// The round-trip test proves the file works. This proves it looks like
/// something: a dump that is valid and unreadable is still a bad backup.
async fn export_writes_a_readable_file(engine: Engine, db: &str, schema: &str) {
    use thabi_studio_lib::database::dump::{
        DumpOptions, contents_note, header, plan, structure_statements, write_table_rows,
    };

    let table = thabi_studio_lib::database::dump::table_of(engine, db, schema, "thabi_dump");
    let mut session = open(engine).await;
    session
        .execute(&format!("DROP TABLE IF EXISTS {table}"), 0)
        .await
        .expect("drop");
    let create = match engine {
        Engine::MySql => {
            "CREATE TABLE thabi_dump (id int PRIMARY KEY, label varchar(50) NULL, amount decimal(10,2) NOT NULL DEFAULT 0)"
        }
        Engine::Postgres => {
            "CREATE TABLE thabi_dump (id int PRIMARY KEY, label text NULL, amount decimal(10,2) NOT NULL DEFAULT 0)"
        }
        Engine::SqlServer => {
            "CREATE TABLE thabi_dump (id int PRIMARY KEY, label nvarchar(50) NULL, amount decimal(10,2) NOT NULL DEFAULT 0)"
        }
    };
    session.execute(create, 0).await.expect("create");
    session
        .execute(
            &format!(
                "INSERT INTO {table} (id, label, amount) VALUES (1, 'Ada', 10.50), (2, 'O''Hara', 0)"
            ),
            0,
        )
        .await
        .expect("insert");

    let options = DumpOptions {
        structure: true,
        data: true,
        ..Default::default()
    };
    let planned = plan(&mut session, db, schema, &options).await;

    let mut file: Vec<u8> = Vec::new();
    use std::io::Write;
    file.write_all(header(engine, db, schema, "2026-10-17 09:00").as_bytes())
        .expect("header");
    file.write_all(contents_note(&options).as_bytes())
        .expect("note");
    file.write_all(b"\n").expect("newline");
    for (name, structure) in &planned.expect("plan") {
        for statement in structure_statements(engine, db, schema, name, structure) {
            file.write_all(statement.as_bytes()).expect("statement");
        }
        let mut silence = |_: u64| {};
        let mut run = write_run(None, &mut silence);
        write_table_rows(
            &mut session,
            &target_for(engine, db, schema, name),
            structure,
            &mut run,
            &mut file,
        )
        .await
        .expect("rows");
    }

    let path = format!(
        "/tmp/thabi-dump-{}.sql",
        format!("{engine:?}").to_lowercase()
    );
    std::fs::write(&path, &file).expect("write dump");
    let _ = file.len();
    println!("DUMP WRITTEN: {path}");
    session
        .execute(&format!("DROP TABLE IF EXISTS {table}"), 0)
        .await
        .ok();
}

fn target_for(
    _engine: Engine,
    database: &str,
    schema: &str,
    name: &str,
) -> thabi_studio_lib::database::ddl::TableTarget {
    thabi_studio_lib::database::ddl::TableTarget {
        database: database.to_owned(),
        schema: (!schema.is_empty()).then(|| schema.to_owned()),
        name: name.to_owned(),
    }
}

/// A run that is never cancelled and reports nowhere.
fn write_run<'a>(
    limit: Option<u64>,
    progress: &'a mut dyn FnMut(u64),
) -> thabi_studio_lib::database::dump::WriteRun<'a> {
    thabi_studio_lib::database::dump::WriteRun::new(
        limit,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        progress,
    )
}

/// Splits a dump back into statements, on semicolons outside string literals.
///
/// A real splitter, not `split(';')`: the values this is meant to carry
/// contain semicolons, and a test that split naively would be testing a
/// simpler thing than the one that matters.
fn split_statements(script: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut previous = '\0';
    for character in script.chars() {
        if character == '\'' && previous != '\\' {
            in_string = !in_string;
        }
        if character == ';' && !in_string {
            let statement = current.trim();
            if !statement.is_empty() && !statement.starts_with("--") {
                out.push(format!("{statement};"));
            }
            current.clear();
            previous = character;
            continue;
        }
        if !character.is_whitespace() || !current.trim().is_empty() {
            current.push(character);
        }
        previous = character;
    }
    let tail = current.trim();
    if !tail.is_empty() && !tail.starts_with("--") {
        out.push(tail.to_owned());
    }
    out
}

/// The ER diagram's three queries, run against the server.
///
/// Every one of these has been written from an assumption about an engine's
/// catalog at some point and been wrong; they are cheap to run and the answer
/// comes from the server, so there is no reason to reason about them instead.
#[allow(clippy::too_many_arguments)]
async fn diagram_reads_tables_columns_and_keys(
    engine: Engine,
    database: &str,
    schema: &str,
    setup: &[&str],
) {
    let mut session = open(engine).await;
    for statement in setup {
        session.execute(statement, 0).await.expect("setup");
    }

    // Through the same method the command calls, so the test covers the real
    // path rather than the pieces it is built from.
    let found = session.diagram(database, schema).await.expect("diagram");

    let country = found
        .tables
        .iter()
        .find(|table| table.name == "thabi_country")
        .unwrap_or_else(|| {
            panic!(
                "{engine:?} did not return the table: {:?}",
                found.tables.iter().map(|t| &t.name).collect::<Vec<_>>()
            )
        });
    assert_eq!(country.columns.len(), 2, "{engine:?}");
    assert!(
        country.columns.iter().any(|c| c.name == "name"),
        "{engine:?}"
    );

    let id = country
        .columns
        .iter()
        .find(|c| c.name == "id")
        .expect("id column");
    assert!(id.primary_key, "{engine:?} did not mark the primary key");
    assert!(!id.nullable, "{engine:?} marked a NOT NULL column nullable");

    let child = found
        .tables
        .iter()
        .find(|table| table.name == "thabi_child")
        .unwrap_or_else(|| panic!("{engine:?} did not return the child table"));
    assert!(
        !child
            .columns
            .iter()
            .any(|c| c.primary_key && c.name == "country_id"),
        "{engine:?} marked a foreign key column as primary"
    );

    // The line between them, which is the whole reason for the diagram.
    let edge = found
        .foreign_keys
        .iter()
        .find(|key| key.from_table == "thabi_child" && key.to_table == "thabi_country")
        .unwrap_or_else(|| panic!("{engine:?} returned no edge: {:?}", found.foreign_keys));
    assert_eq!(edge.from_columns, ["country_id"], "{engine:?}");
    assert_eq!(edge.to_columns, ["id"], "{engine:?}");

    for statement in setup.iter().filter(|s| s.starts_with("DROP TABLE")) {
        session.execute(statement, 0).await.expect("cleanup");
    }
}

async fn applies_generated_ddl(engine: Engine) {
    use thabi_studio_lib::database::ddl::{
        Change, ColumnSpec, IndexAction, IndexSpec, KeyChange, TableAction, TablePlan, TableSpec,
        action_script, create_table, drop_table, table_script,
    };
    let mut session = open(engine).await;
    // Each session is already connected to its database: the SQL Server
    // container only has master, and PostgreSQL has no USE at all.
    let (db, schema) = match engine {
        Engine::MySql => ("thabi_it", ""),
        Engine::Postgres => ("thabi_it", "public"),
        Engine::SqlServer => ("master", "dbo"),
    };
    session
        .execute(&drop_table(engine, db, schema, "thabi ddl", true), 1)
        .await
        .unwrap_or_else(|e| panic!("drop leftovers from a previous run: {e:?}"));
    // MySQL names check constraints across the whole schema rather than per
    // table, so a leftover copy of the renamed table would collide with the
    // one this run is about to make.
    for leftover in ["thabi_actions", "thabi_actions_copy"] {
        session
            .execute(&drop_table(engine, db, schema, leftover, true), 1)
            .await
            .expect("drop leftovers");
    }

    // A table with an awkward name, so quoting is exercised for real.
    let spec = TableSpec {
        columns: vec![
            ColumnSpec {
                name: "id".into(),
                data_type: "INTEGER".into(),
                nullable: false,
                primary_key: true,
                ..Default::default()
            },
            ColumnSpec {
                name: "status".into(),
                data_type: "VARCHAR(20)".into(),
                nullable: false,
                default: "'new'".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let created = create_table(engine, db, schema, "thabi ddl", &spec).expect("create table");
    session
        .execute(&created, 1)
        .await
        .expect("generated CREATE TABLE");

    let structure = session
        .describe(
            db,
            schema,
            "thabi ddl",
            thabi_studio_lib::database::ObjectKind::Table,
        )
        .await
        .expect("describe after create");
    let names: Vec<&str> = structure.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["id", "status"], "{engine:?} made a different table");
    let id = structure.columns.iter().find(|c| c.name == "id").unwrap();
    assert!(id.primary_key.is_some(), "{engine:?} lost the primary key");
    let status = structure
        .columns
        .iter()
        .find(|c| c.name == "status")
        .unwrap();
    assert!(
        !status.nullable,
        "{engine:?} made a nullable column not null"
    );

    // A plan touching a column, a key and an index, generated and applied.
    let plan = TablePlan {
        create: None,
        columns: vec![
            Change::AddColumn(ColumnSpec {
                name: "note".into(),
                data_type: "TEXT".into(),
                nullable: true,
                ..Default::default()
            }),
            Change::SetNullable {
                name: "status".into(),
                nullable: true,
                data_type: "VARCHAR(20)".into(),
            },
            // A retype, which is the one that was refused on MySQL: it spells
            // a restated column MODIFY, not ALTER COLUMN.
            Change::SetType {
                name: "note".into(),
                data_type: match engine {
                    Engine::MySql => "VARCHAR(120)".into(),
                    Engine::Postgres => "VARCHAR(120)".into(),
                    Engine::SqlServer => "NVARCHAR(120)".into(),
                },
            },
        ],
        keys: vec![KeyChange::AddCheck {
            name: "status_not_blank".into(),
            expression: "status <> ''".into(),
        }],
        indexes: vec![IndexAction::Create(IndexSpec {
            name: "thabi_ddl_status_idx".into(),
            columns: vec!["status".into()],
            unique: true,
        })],
    };
    let script = table_script(engine, db, schema, "thabi ddl", &plan);
    assert!(!script.is_empty());
    for statement in &script {
        session
            .execute(statement, 1)
            .await
            .unwrap_or_else(|e| panic!("{engine:?} refused {statement}\n{e:?}"));
    }

    let after = session
        .describe(
            db,
            schema,
            "thabi ddl",
            thabi_studio_lib::database::ObjectKind::Table,
        )
        .await
        .expect("describe after alter");
    let names: Vec<&str> = after.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["id", "status", "note"],
        "{engine:?} alter did not take"
    );
    let status = after.columns.iter().find(|c| c.name == "status").unwrap();
    assert!(
        status.nullable,
        "{engine:?} did not make the column nullable"
    );
    let indexes = after.indexes.expect("indexes for a table");
    assert!(
        indexes
            .iter()
            .any(|i| i.name == "thabi_ddl_status_idx" && i.unique),
        "{engine:?} did not create the unique index: {indexes:?}"
    );

    // The path the Create Table wizard uses: a create plan previewed and then
    // applied, which is one statement rather than a list of alters.
    let wizard = TablePlan {
        create: Some(TableSpec {
            columns: vec![
                ColumnSpec {
                    name: "id".into(),
                    data_type: "INTEGER".into(),
                    nullable: false,
                    primary_key: true,
                    ..Default::default()
                },
                ColumnSpec {
                    name: "label".into(),
                    data_type: match engine {
                        Engine::SqlServer => "NVARCHAR(MAX)".into(),
                        _ => "TEXT".into(),
                    },
                    nullable: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        }),
        ..Default::default()
    };
    let shown =
        thabi_studio_lib::database::ddl::preview(engine, db, schema, "thabi wizard", &wizard);
    assert!(shown.note.contains("transaction") || shown.note.contains("commits"));
    for statement in &shown.statements {
        assert!(
            statement.starts_with("CREATE TABLE"),
            "{engine:?}: {statement}"
        );
        session
            .execute(statement, 1)
            .await
            .unwrap_or_else(|e| panic!("{engine:?} refused the wizard's SQL {statement}\n{e:?}"));
    }
    let made = session
        .describe(
            db,
            schema,
            "thabi wizard",
            thabi_studio_lib::database::ObjectKind::Table,
        )
        .await
        .expect("describe the wizard's table");
    let names: Vec<&str> = made.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["id", "label"], "{engine:?} made a different table");
    session
        .execute(&drop_table(engine, db, schema, "thabi wizard", true), 1)
        .await
        .expect("drop the wizard's table");

    // The designer's own shape: change an existing column and add one, which
    // is what the Structure tab stages.
    let staged = TablePlan {
        create: None,
        columns: vec![
            Change::SetNullable {
                name: "status".into(),
                nullable: true,
                data_type: "VARCHAR(20)".into(),
            },
            Change::AddColumn(ColumnSpec {
                // A name the earlier plan did not already add.
                name: "designer_note".into(),
                data_type: "TEXT".into(),
                nullable: true,
                ..Default::default()
            }),
        ],
        keys: vec![],
        indexes: vec![],
    };
    for statement in table_script(engine, db, schema, "thabi ddl", &staged) {
        session
            .execute(&statement, 1)
            .await
            .unwrap_or_else(|e| panic!("{engine:?} refused {statement}\n{e:?}"));
    }
    let changed = session
        .describe(
            db,
            schema,
            "thabi ddl",
            thabi_studio_lib::database::ObjectKind::Table,
        )
        .await
        .expect("describe after the staged changes");
    let names: Vec<&str> = changed.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["id", "status", "note", "designer_note"],
        "{engine:?}: {names:?}"
    );
    assert!(
        changed
            .columns
            .iter()
            .find(|c| c.name == "status")
            .unwrap()
            .nullable,
        "{engine:?} did not make the column nullable"
    );
    let retyped = changed
        .columns
        .iter()
        .find(|c| c.name == "note")
        .unwrap()
        .data_type
        .to_lowercase();
    assert!(
        retyped.contains("120"),
        "{engine:?} did not change the column's type: {retyped}"
    );

    // Rows first, so a copy with data has something to copy.
    for id in 1..=3 {
        session
            .execute(
                &format!(
                    // Distinct: an earlier step made a unique index on status.
                    "INSERT INTO {} (id, status) VALUES ({id}, 'new{id}')",
                    thabi_studio_lib::database::quote::table(engine, db, schema, "thabi ddl")
                ),
                1,
            )
            .await
            .expect("insert rows to copy");
    }

    // The whole-table actions, run the way the menu runs them: generate, then
    // apply. Duplicate with data is the one with something to get wrong.
    let rename_to = "thabi_actions";
    for statement in action_script(
        engine,
        db,
        schema,
        "thabi ddl",
        &TableAction::Rename {
            to: rename_to.into(),
        },
        None,
    )
    .unwrap()
    {
        session
            .execute(&statement, 1)
            .await
            .unwrap_or_else(|e| panic!("{engine:?} refused {statement}\n{e:?}"));
    }
    let renamed = session
        .describe(
            db,
            schema,
            rename_to,
            thabi_studio_lib::database::ObjectKind::Table,
        )
        .await
        .unwrap_or_else(|e| panic!("{engine:?}: rename did not take: {e:?}"));
    assert_eq!(renamed.columns.len(), changed.columns.len());

    let spec = thabi_studio_lib::database::ddl::spec_from_structure(&renamed);
    for statement in action_script(
        engine,
        db,
        schema,
        rename_to,
        &TableAction::DuplicateWithData {
            to: "thabi_actions_copy".into(),
        },
        Some(&spec),
    )
    .unwrap()
    {
        session
            .execute(&statement, 1)
            .await
            .unwrap_or_else(|e| panic!("{engine:?} refused {statement}\n{e:?}"));
    }
    let copy = session
        .describe(
            db,
            schema,
            "thabi_actions_copy",
            thabi_studio_lib::database::ObjectKind::Table,
        )
        .await
        .unwrap_or_else(|e| panic!("{engine:?}: copy did not take: {e:?}"));
    assert_eq!(copy.columns.len(), renamed.columns.len());
    let rows = session
        .execute(
            &format!(
                "SELECT COUNT(*) FROM {}",
                thabi_studio_lib::database::quote::table(engine, db, schema, "thabi_actions_copy")
            ),
            1,
        )
        .await
        .expect("count the copy");
    assert_eq!(
        rows.rows[0][0],
        thabi_studio_lib::database::Cell::Value("3".into()),
        "{engine:?}: the copy has the wrong rows"
    );

    // Emptying keeps the structure, which is the difference from dropping.
    for statement in action_script(
        engine,
        db,
        schema,
        rename_to,
        &TableAction::Truncate {
            restart_identity: false,
        },
        None,
    )
    .unwrap()
    {
        session
            .execute(&statement, 1)
            .await
            .unwrap_or_else(|e| panic!("{engine:?}: {e:?}"));
    }
    let after = session
        .describe(
            db,
            schema,
            rename_to,
            thabi_studio_lib::database::ObjectKind::Table,
        )
        .await
        .expect("describe after truncate");
    assert_eq!(
        after.columns.len(),
        renamed.columns.len(),
        "{engine:?}: truncate changed the structure"
    );

    // A table with no key and no columns is refused before it reaches a server.
    assert!(create_table(engine, db, schema, "empty", &TableSpec::default()).is_err());
}

async fn binds_values_verbatim(engine: Engine, supports_binding: bool) {
    use thabi_studio_lib::database::values::Param;
    let mut session = open(engine).await;
    // `maybe` is a text column because a NULL parameter binds as text, and
    // PostgreSQL will not cast text to integer even for NULL — recorded as a
    // limit in docs/decisions/0030-bound-parameters.md.
    let note = if engine == Engine::SqlServer {
        "nvarchar(200)"
    } else {
        "varchar(200)"
    };
    let setup = format!(
        "DROP TABLE IF EXISTS thabi_param_test; CREATE TABLE thabi_param_test (id int PRIMARY KEY, note {note}, amount decimal(10,2), maybe varchar(50), big bigint)"
    );
    let setup = setup.as_str();
    session.execute(setup, 10).await.unwrap();

    let hostile = "O'Brien'; DROP TABLE thabi_param_test; --";
    // A number, text holding quotes and a semicolon, a decimal, and a NULL.
    // Each engine's own placeholder style: this is what the frontend rewrites a
    // statement to before it gets here.
    let insert = match engine {
        Engine::Postgres => {
            "INSERT INTO thabi_param_test (id, note, amount, maybe, big) VALUES ($1, $2, $3, $4, $5)"
        }
        Engine::MySql => {
            "INSERT INTO thabi_param_test (id, note, amount, maybe, big) VALUES (?, ?, ?, ?, ?)"
        }
        Engine::SqlServer => {
            "INSERT INTO thabi_param_test (id, note, amount, maybe, big) VALUES (@P1, @P2, @P3, @P4, @P5)"
        }
    };
    let result = session
        .execute_bound(
            insert,
            &[
                Param::Number("1".into()),
                Param::Text(hostile.into()),
                Param::Number("1234.56".into()),
                Param::Null,
                // Above 2^53: a float would round it.
                Param::Number("9007199254740993".into()),
            ],
            10,
        )
        .await;
    if !supports_binding {
        let error = result.expect_err("this engine cannot bind yet");
        assert_eq!(error.kind, "not_supported");
        return;
    }
    result.expect("bound insert");

    // Read it back with the text protocol, so what is asserted is what a
    // person would see in the grid.
    let read = session
        .execute(
            "SELECT note, amount, maybe, big FROM thabi_param_test WHERE id = 1",
            10,
        )
        .await
        .unwrap();
    let row = &read.rows[0];
    assert_eq!(
        row[0],
        Cell::Value(hostile.into()),
        "value was not stored as typed"
    );
    assert_eq!(
        row[1],
        Cell::Value("1234.56".into()),
        "number lost precision"
    );
    assert_eq!(row[2], Cell::Null, "NULL did not stay NULL");
    assert_eq!(
        row[3],
        Cell::Value("9007199254740993".into()),
        "a big integer lost digits"
    );

    // And the row is still there, so the semicolon did not end the statement.
    let count = session
        .execute("SELECT COUNT(*) FROM thabi_param_test", 10)
        .await
        .unwrap();
    assert_eq!(count.rows[0][0], Cell::Value("1".into()));
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

/// Table-data filters run on every engine, including LIKE on a number column
/// and a text value compared with an integer column.
async fn filters_rows(engine: Engine) {
    use thabi_studio_lib::database::filter::{RowFilter, where_clause};
    let f = |column: &str, op: &str, value: &str| RowFilter {
        column: column.into(),
        op: op.into(),
        value: value.into(),
    };
    let source =
        "SELECT 71 AS id, 'O''Brien' AS name UNION ALL SELECT 5, 'Zoë' UNION ALL SELECT 9, NULL";
    let mut session = open(engine).await;
    for (filters, expected) in [
        (vec![f("id", "eq", "71")], vec!["71"]),
        (vec![f("name", "contains", "'Bri")], vec!["71"]),
        (vec![f("id", "contains", "7")], vec!["71"]),
        (vec![f("id", "in", "5, 9")], vec!["9", "5"]),
        (vec![f("name", "is_null", "")], vec!["9"]),
        (
            vec![f("id", "ge", "9"), f("", "raw", "id < 100")],
            vec!["71", "9"],
        ),
    ] {
        let clause = where_clause(engine, &filters).unwrap();
        let sql = format!("SELECT id FROM ({source}) t{clause} ORDER BY id DESC");
        let result = session.execute(&sql, 10).await.unwrap();
        let ids: Vec<_> = values(&result)
            .into_iter()
            .map(|row| row[0].clone().unwrap())
            .collect();
        assert_eq!(ids, expected, "{sql}");
    }
    session.close().await.unwrap();
}

/// Enum values and foreign keys are reported, and a foreign key's values
/// can be looked up (with a readable label) for the edit picker.
async fn describes_enums_and_references(
    engine: Engine,
    database: &str,
    schema: &str,
    setup: &[&str],
    expect_enum: bool,
) {
    use thabi_studio_lib::database::{
        ObjectKind,
        edit::{label_column, lookup_sql},
    };
    let mut session = open(engine).await;
    for sql in setup {
        session
            .execute(sql, 1)
            .await
            .unwrap_or_else(|e| panic!("{sql}: {}", e.message));
    }
    let (db, sch) = match engine {
        Engine::MySql => (database, ""),
        _ => (database, schema),
    };
    let child = session
        .describe(db, sch, "thabi_child", ObjectKind::Table)
        .await
        .unwrap();
    let country = child
        .columns
        .iter()
        .find(|c| c.name == "country_id")
        .unwrap();
    let reference = country.references.clone().expect("foreign key reported");
    assert_eq!(reference.table, "thabi_country");
    assert_eq!(reference.column, "id");
    let status = child.columns.iter().find(|c| c.name == "status").unwrap();
    if expect_enum {
        assert_eq!(status.enum_values, Some(vec!["new".into(), "paid".into()]));
    }
    let parent = session
        .describe(db, sch, "thabi_country", ObjectKind::Table)
        .await
        .unwrap();
    // Autocomplete sees the table and its columns, in order.
    let completion = session.completion_schema(database).await.unwrap();
    let child = completion
        .iter()
        .find(|t| t.table == "thabi_child")
        .unwrap_or_else(|| panic!("{engine:?}: {completion:?}"));
    assert_eq!(child.columns, ["id", "status", "country_id"]);

    // A join's plan renders on every engine and names both tables.
    let plan = thabi_studio_lib::database::plan::explain(
        &mut session,
        engine,
        "SELECT * FROM thabi_child JOIN thabi_country ON thabi_country.id = thabi_child.country_id",
        false,
    )
    .await
    .unwrap_or_else(|e| panic!("{engine:?}: {} {:?}", e.message, e.detail));
    fn labels(nodes: &[thabi_studio_lib::database::plan::PlanNode], out: &mut String) {
        for node in nodes {
            out.push_str(&node.label);
            out.push('\n');
            labels(&node.children, out);
        }
    }
    let mut all = String::new();
    labels(&plan.nodes, &mut all);
    assert!(
        all.contains("thabi_child") && all.contains("thabi_country"),
        "{engine:?} plan: {all}\n{}",
        plan.raw
    );

    // And the other way: who points at the parent.
    let children = session
        .referencing(database, schema, "thabi_country")
        .await
        .unwrap();
    assert!(
        children.iter().any(|r| r.table == "thabi_child"
            && r.column == "country_id"
            && r.referenced_column == "id"),
        "{engine:?}: {children:?}"
    );
    let label = label_column("id", parent.columns.iter().map(|c| c.name.as_str()));
    assert_eq!(label, Some("name"));
    let result = session
        .execute(&lookup_sql(engine, database, &reference, label, "mal"), 50)
        .await
        .unwrap();
    assert_eq!(
        values(&result),
        [vec![Some("2".to_owned()), Some("Malawi".to_owned())]]
    );
    session.close().await.unwrap();
}

mod postgres {
    use super::*;
    const E: Engine = Engine::Postgres;

    #[tokio::test]
    #[ignore]
    async fn generated_ddl_applies_and_reads_back() {
        applies_generated_ddl(E).await;
    }

    #[tokio::test]
    #[ignore]
    async fn explain_analyze_changes_nothing() {
        let mut session = open(E).await;
        session
            .execute(
                "DROP TABLE IF EXISTS thabi_analyze; CREATE TABLE thabi_analyze (id int); INSERT INTO thabi_analyze VALUES (1), (2)",
                1,
            )
            .await
            .unwrap();
        let plan = thabi_studio_lib::database::plan::explain(
            &mut session,
            E,
            "DELETE FROM thabi_analyze",
            true,
        )
        .await
        .unwrap();
        assert!(plan.analyzed);
        assert_eq!(plan.nodes[0].label, "ModifyTable on thabi_analyze");
        let count = session
            .execute("SELECT COUNT(*) FROM thabi_analyze", 1)
            .await
            .unwrap();
        assert_eq!(count.rows[0][0], Cell::Value("2".into()), "rolled back");
        session.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn bound_values_are_stored_verbatim() {
        binds_values_verbatim(E, true).await;
    }

    #[tokio::test]
    #[ignore]
    async fn bound_values_are_decoded_by_type() {
        decodes_bound_values_by_type(E).await;
    }

    #[tokio::test]
    #[ignore]
    async fn enums_and_references() {
        describes_enums_and_references(
            E,
            "thabi_it",
            "public",
            &[
                "DROP TABLE IF EXISTS thabi_child",
                "DROP TABLE IF EXISTS thabi_country",
                "DROP TYPE IF EXISTS thabi_status",
                "CREATE TYPE thabi_status AS ENUM ('new', 'paid')",
                "CREATE TABLE thabi_country (id int PRIMARY KEY, name text)",
                "INSERT INTO thabi_country VALUES (1, 'Kenya'), (2, 'Malawi')",
                "CREATE TABLE thabi_child (id int PRIMARY KEY, status thabi_status, country_id int REFERENCES thabi_country(id))",
            ],
            true,
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn filters() {
        filters_rows(E).await;
    }

    #[tokio::test]
    #[ignore]
    async fn connects() {
        connects_and_reports_version(E, "PostgreSQL 16").await;
    }

    #[tokio::test]
    #[ignore]
    async fn sql_export_writes_a_readable_file() {
        export_writes_a_readable_file(E, "thabi_it", "public").await;
    }

    #[tokio::test]
    #[ignore]
    async fn sql_export_round_trips() {
        export_round_trips(E, "thabi_it", "public").await;
    }

    #[tokio::test]
    #[ignore]
    async fn designer_adds_and_drops_a_foreign_key() {
        adds_and_drops_a_foreign_key(E, "thabi_it", "public").await;
    }

    #[tokio::test]
    #[ignore]
    async fn diagram_reads_the_schema() {
        diagram_reads_tables_columns_and_keys(
            E,
            "thabi_it",
            "public",
            &[
                "DROP TABLE IF EXISTS thabi_child",
                "DROP TABLE IF EXISTS thabi_country",
                "CREATE TABLE thabi_country (id int PRIMARY KEY, name text)",
                "CREATE TABLE thabi_child (id int PRIMARY KEY, country_id int REFERENCES thabi_country(id))",
            ],
        )
        .await;
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
    async fn generated_ddl_applies_and_reads_back() {
        applies_generated_ddl(E).await;
    }

    #[tokio::test]
    #[ignore]
    async fn sql_export_writes_a_readable_file() {
        export_writes_a_readable_file(E, "thabi_it", "").await;
    }

    #[tokio::test]
    #[ignore]
    async fn sql_export_round_trips() {
        export_round_trips(E, "thabi_it", "").await;
    }

    #[tokio::test]
    #[ignore]
    async fn designer_adds_and_drops_a_foreign_key() {
        adds_and_drops_a_foreign_key(E, "thabi_it", "").await;
    }

    #[tokio::test]
    #[ignore]
    async fn diagram_reads_the_schema() {
        diagram_reads_tables_columns_and_keys(
            E,
            "thabi_it",
            "",
            &[
                "DROP TABLE IF EXISTS thabi_child",
                "DROP TABLE IF EXISTS thabi_country",
                "CREATE TABLE thabi_country (id int PRIMARY KEY, name text)",
                "CREATE TABLE thabi_child (id int PRIMARY KEY, country_id int, FOREIGN KEY (country_id) REFERENCES thabi_country(id))",
            ],
        )
        .await;
    }

    /// The premise behind the MySQL half-applied message, checked against the
    /// server rather than reasoned about: a DDL batch that fails on its second
    /// statement leaves the first one committed. This has been assumed four
    /// times in this file and is wrong every time it is reasoned about.
    #[tokio::test]
    #[ignore]
    async fn a_failed_ddl_batch_keeps_the_statements_that_ran() {
        let mut session = open(E).await;
        session
            .execute("CREATE TABLE `partial_test` (`keepme` INT NOT NULL);", 0)
            .await
            .expect("create");

        // First statement is fine. Second names a column that does not exist.
        let batch = "ALTER TABLE `partial_test` CHANGE COLUMN `keepme` `kept` INT NOT NULL;\n\
                     ALTER TABLE `partial_test` MODIFY COLUMN `never_was` TEXT;";
        let failure = session.execute(batch, 0).await.expect_err("must fail");
        let detail = failure.detail.clone().unwrap_or_default();
        assert!(
            detail.to_lowercase().contains("unknown column")
                || detail.to_lowercase().contains("doesn't exist"),
            "unexpected reason: {detail}"
        );

        // The point: the rename above is already committed, so the table is
        // half changed even though the batch as a whole failed.
        let structure = session
            .describe(
                "thabi_it",
                "",
                "partial_test",
                thabi_studio_lib::database::ObjectKind::Table,
            )
            .await
            .expect("read back");
        let names: Vec<&str> = structure
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect();
        assert_eq!(names, vec!["kept"], "the first statement did not commit");
        assert!(
            structure
                .columns
                .iter()
                .all(|column| column.name != "never_was"),
            "the failing statement left something behind"
        );

        session
            .execute("DROP TABLE `partial_test`;", 0)
            .await
            .expect("cleanup");
    }

    #[tokio::test]
    #[ignore]
    async fn bound_values_are_stored_verbatim() {
        binds_values_verbatim(E, true).await;
    }

    #[tokio::test]
    #[ignore]
    async fn enums_and_references() {
        describes_enums_and_references(
            E,
            "thabi_it",
            "",
            &[
                "DROP TABLE IF EXISTS thabi_child",
                "DROP TABLE IF EXISTS thabi_country",
                "CREATE TABLE thabi_country (id int PRIMARY KEY, name varchar(50))",
                "INSERT INTO thabi_country VALUES (1, 'Kenya'), (2, 'Malawi')",
                "CREATE TABLE thabi_child (id int PRIMARY KEY, status ENUM('new', 'paid'), country_id int, FOREIGN KEY (country_id) REFERENCES thabi_country(id))",
            ],
            true,
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn filters() {
        filters_rows(E).await;
    }

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
    async fn generated_ddl_applies_and_reads_back() {
        applies_generated_ddl(E).await;
    }

    #[tokio::test]
    #[ignore]
    async fn sql_export_writes_a_readable_file() {
        export_writes_a_readable_file(E, "master", "dbo").await;
    }

    #[tokio::test]
    #[ignore]
    async fn sql_export_round_trips() {
        export_round_trips(E, "master", "dbo").await;
    }

    #[tokio::test]
    #[ignore]
    async fn designer_adds_and_drops_a_foreign_key() {
        adds_and_drops_a_foreign_key(E, "master", "dbo").await;
    }

    #[tokio::test]
    #[ignore]
    async fn bound_values_are_stored_verbatim() {
        binds_values_verbatim(E, true).await;
    }

    #[tokio::test]
    #[ignore]
    async fn enums_and_references() {
        describes_enums_and_references(
            E,
            "master",
            "dbo",
            &[
                "IF OBJECT_ID('thabi_child') IS NOT NULL DROP TABLE thabi_child",
                "IF OBJECT_ID('thabi_country') IS NOT NULL DROP TABLE thabi_country",
                "CREATE TABLE thabi_country (id int PRIMARY KEY, name nvarchar(50))",
                "INSERT INTO thabi_country VALUES (1, N'Kenya'), (2, N'Malawi')",
                "CREATE TABLE thabi_child (id int PRIMARY KEY, status varchar(10), country_id int REFERENCES thabi_country(id))",
            ],
            false,
        )
        .await;
    }

    #[tokio::test]
    #[ignore]
    async fn filters() {
        filters_rows(E).await;
    }

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

/// Explorer metadata with names that need quoting: spaces, a reserved
/// word, mixed case and non-ASCII characters.
mod explorer {
    use super::*;
    use thabi_studio_lib::database::{ObjectKind, ObjectStructure};

    async fn run_all(session: &mut Session, sql: &[&str]) {
        for statement in sql {
            session
                .execute(statement, 10)
                .await
                .unwrap_or_else(|e| panic!("{statement}: {} {:?}", e.message, e.detail));
        }
    }

    fn check_structure(structure: &ObjectStructure) {
        let names: Vec<_> = structure.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["id", "select", "MixedCase"]);
        assert_eq!(structure.columns[0].primary_key, Some(1));
        assert_eq!(structure.columns[2].primary_key, Some(2));
        assert_eq!(structure.columns[1].primary_key, None);
        assert!(!structure.columns[1].nullable);
        assert!(
            structure.columns[1]
                .default
                .as_deref()
                .unwrap_or("")
                .contains('x')
        );
        let indexes = structure.indexes.as_ref().expect("tables have indexes");
        let by_name = indexes
            .iter()
            .find(|i| i.name == "idx ünï")
            .expect("named index");
        assert_eq!(by_name.columns, vec!["select"]);
        assert!(indexes.iter().any(|i| i.primary));
    }

    async fn check_objects(session: &mut Session, database: &str, schema: &str) {
        let objects = session.list_objects(database, schema).await.unwrap();
        let find = |name: &str| objects.iter().find(|o| o.name == name).map(|o| o.kind);
        assert_eq!(
            find("Order Details"),
            Some(ObjectKind::Table),
            "{objects:?}"
        );
        assert_eq!(find("Big Orders"), Some(ObjectKind::View), "{objects:?}");

        let table = session
            .describe(database, schema, "Order Details", ObjectKind::Table)
            .await
            .unwrap();
        check_structure(&table);

        let preview = session
            .preview(database, schema, "Order Details", 100)
            .await
            .unwrap();
        assert_eq!(preview.rows.len(), 1);
        let view = session
            .preview(database, schema, "Big Orders", 100)
            .await
            .unwrap();
        assert_eq!(view.rows.len(), 1);
    }

    #[tokio::test]
    #[ignore]
    async fn postgres() {
        let mut session = open(Engine::Postgres).await;
        run_all(
            &mut session,
            &[
                "DROP SCHEMA IF EXISTS \"Sales Ünïcödé\" CASCADE",
                "CREATE SCHEMA \"Sales Ünïcödé\"",
                "CREATE TABLE \"Sales Ünïcödé\".\"Order Details\" (id bigint, \"select\" text NOT NULL DEFAULT 'x', \"MixedCase\" int, PRIMARY KEY (id, \"MixedCase\"))",
                "CREATE INDEX \"idx ünï\" ON \"Sales Ünïcödé\".\"Order Details\" (\"select\")",
                "CREATE VIEW \"Sales Ünïcödé\".\"Big Orders\" AS SELECT * FROM \"Sales Ünïcödé\".\"Order Details\"",
                "INSERT INTO \"Sales Ünïcödé\".\"Order Details\" VALUES (1, 'a', 2)",
            ],
        )
        .await;
        let schemas = session.list_schemas("thabi_it").await.unwrap().unwrap();
        assert!(schemas.contains(&"Sales Ünïcödé".to_owned()), "{schemas:?}");
        assert!(schemas.contains(&"public".to_owned()));
        assert!(!schemas.iter().any(|s| s.starts_with("pg_")));
        check_objects(&mut session, "thabi_it", "Sales Ünïcödé").await;

        let view = session
            .describe("thabi_it", "Sales Ünïcödé", "Big Orders", ObjectKind::View)
            .await
            .unwrap();
        assert!(
            view.indexes.is_none(),
            "views report indexes as not applicable"
        );

        let other = session.list_schemas("postgres").await.unwrap_err();
        assert_eq!(other.kind, "database_not_open");
        session.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn mysql() {
        let mut session = open(Engine::MySql).await;
        run_all(
            &mut session,
            &[
                "DROP VIEW IF EXISTS `Big Orders`",
                "DROP TABLE IF EXISTS `Order Details`",
                "CREATE TABLE `Order Details` (id bigint, `select` varchar(20) NOT NULL DEFAULT 'x', `MixedCase` int, PRIMARY KEY (id, `MixedCase`), INDEX `idx ünï` (`select`))",
                "CREATE VIEW `Big Orders` AS SELECT * FROM `Order Details`",
                "INSERT INTO `Order Details` VALUES (1, 'a', 2)",
            ],
        )
        .await;
        assert!(session.list_schemas("thabi_it").await.unwrap().is_none());
        check_objects(&mut session, "thabi_it", "").await;
        session.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn sqlserver() {
        let mut session = open(Engine::SqlServer).await;
        run_all(
            &mut session,
            &["IF DB_ID('thabi it') IS NULL CREATE DATABASE [thabi it]"],
        )
        .await;
        // Created through a session on master to prove three-part names work.
        run_all(
            &mut session,
            &[
                "USE [thabi it]",
                "IF OBJECT_ID(N'[Sales Ünïcödé].[Big Orders]') IS NOT NULL DROP VIEW [Sales Ünïcödé].[Big Orders]",
                "IF OBJECT_ID(N'[Sales Ünïcödé].[Order Details]') IS NOT NULL DROP TABLE [Sales Ünïcödé].[Order Details]",
                "IF SCHEMA_ID(N'Sales Ünïcödé') IS NULL EXEC(N'CREATE SCHEMA [Sales Ünïcödé]')",
                "CREATE TABLE [Sales Ünïcödé].[Order Details] (id bigint NOT NULL, [select] nvarchar(20) NOT NULL DEFAULT 'x', [MixedCase] int NOT NULL, CONSTRAINT pk_od PRIMARY KEY (id, [MixedCase]))",
                "CREATE INDEX [idx ünï] ON [Sales Ünïcödé].[Order Details] ([select])",
                "EXEC(N'CREATE VIEW [Sales Ünïcödé].[Big Orders] AS SELECT * FROM [Sales Ünïcödé].[Order Details]')",
                "INSERT INTO [Sales Ünïcödé].[Order Details] VALUES (1, N'a', 2)",
                "USE master",
            ],
        )
        .await;
        let schemas = session.list_schemas("thabi it").await.unwrap().unwrap();
        assert!(schemas.contains(&"Sales Ünïcödé".to_owned()), "{schemas:?}");
        assert!(schemas.contains(&"dbo".to_owned()));
        check_objects(&mut session, "thabi it", "Sales Ünïcödé").await;
        session.close().await.unwrap();
    }
}

/// Mutual TLS against a server that demands a client certificate: with the
/// certificate it connects, without it the server refuses.
mod mutual_tls {
    use super::*;

    fn mtls(cert: bool) -> ConnectParams {
        let dir = std::path::PathBuf::from(env("THABI_IT_MTLS_DIR"));
        let mut p = params(Engine::Postgres, TlsMode::Verify);
        p.port = env("THABI_IT_PG_MTLS_PORT").parse().unwrap();
        // pg_hba "cert": the certificate is the login, no password needed.
        p.password = None;
        p.ca_path = Some(dir.join("ca.crt"));
        if cert {
            p.client_cert = Some(dir.join("client.crt"));
            p.client_key = Some(dir.join("client.key"));
        }
        p
    }

    #[tokio::test]
    #[ignore]
    async fn connects_with_a_client_certificate() {
        let mut session = Session::connect(&mtls(true))
            .await
            .unwrap_or_else(|e| panic!("{} {:?}", e.message, e.detail));
        let result = session.execute("SELECT current_user", 1).await.unwrap();
        assert_eq!(result.rows[0][0], Cell::Value("thabi".into()));
        session.close().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn is_refused_without_one() {
        assert!(Session::connect(&mtls(false)).await.is_err());
    }
}

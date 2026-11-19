//! Importing files into tables, against real databases.
//! `#[ignore]`d; run with `npm run test:integration`.

use thabi_studio_lib::database::import::{
    ColumnMapping, Encoding, Format, ImportOptions, ImportProgress, ImportTarget, ReadOptions, run,
};
use thabi_studio_lib::database::{Cell, ConnectParams, Engine, Session, TlsMode};

fn env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} is not set; run npm run test:integration"))
}

async fn open(engine: Engine) -> Session {
    let (port, username, database) = match engine {
        Engine::Postgres => (env("THABI_IT_PG_PORT"), "thabi", "thabi_it"),
        Engine::MySql => (env("THABI_IT_MYSQL_PORT"), "thabi", "thabi_it"),
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

fn place(engine: Engine) -> (&'static str, &'static str) {
    match engine {
        Engine::MySql => ("thabi_it", ""),
        Engine::Postgres => ("thabi_it", "public"),
        Engine::SqlServer => ("master", "dbo"),
    }
}

fn file(name: &str, contents: &str) -> String {
    let path = std::env::temp_dir().join(format!("thabi-import-{}-{name}", std::process::id()));
    std::fs::write(&path, contents).unwrap();
    path.to_string_lossy().into_owned()
}

async fn fresh_table(session: &mut Session, engine: Engine) {
    let _ = session
        .execute("DROP TABLE IF EXISTS thabi_import", 0)
        .await;
    let create = match engine {
        Engine::SqlServer => {
            "CREATE TABLE thabi_import (id INT NOT NULL, name NVARCHAR(40), amount DECIMAL(12,2), born DATE, ok BIT)"
        }
        Engine::MySql => {
            "CREATE TABLE thabi_import (id INT NOT NULL, name VARCHAR(40), amount DECIMAL(12,2), born DATE, ok BOOLEAN)"
        }
        Engine::Postgres => {
            "CREATE TABLE thabi_import (id INT NOT NULL, name VARCHAR(40), amount NUMERIC(12,2), born DATE, ok BOOLEAN)"
        }
    };
    session.execute(create, 0).await.unwrap();
}

fn options(skip: bool) -> ImportOptions {
    ImportOptions {
        read: ReadOptions {
            format: Format::Csv,
            delimiter: String::new(),
            quote: "\"".into(),
            header: true,
            encoding: Encoding::Utf8,
        },
        empty_as_null: true,
        skip_bad_rows: skip,
    }
}

fn mappings() -> Vec<ColumnMapping> {
    ["id", "name", "amount", "born", "ok"]
        .iter()
        .enumerate()
        .map(|(i, column)| ColumnMapping {
            file_column: i,
            column: (*column).into(),
        })
        .collect()
}

async fn count(session: &mut Session) -> String {
    let result = session
        .execute("SELECT COUNT(*) FROM thabi_import", 1)
        .await
        .unwrap();
    match &result.rows[0][0] {
        Cell::Value(v) => v.clone(),
        other => format!("{other:?}"),
    }
}

async fn imports_values_and_handles_a_bad_row(engine: Engine) {
    let mut session = open(engine).await;
    let (db, schema) = place(engine);
    let target = ImportTarget {
        database: db.into(),
        schema: (!schema.is_empty()).then(|| schema.into()),
        table: "thabi_import".into(),
    };
    let path = file(
        &format!("{engine:?}.csv"),
        "id,name,amount,born,ok\n\
         1,\"O'Hara, Jr\",12.50,2026-10-20,true\n\
         2,naïve ☕,,,false\n\
         3,\"two\nlines\",1,2001-01-01,true\n\
         oops,bad row,1,2001-01-01,true\n\
         5,after,2,2002-02-02,false\n",
    );

    // Stop: nothing is kept, and the bad row is named by its line.
    fresh_table(&mut session, engine).await;
    let progress = ImportProgress::default();
    let summary = run(
        &mut session,
        &target,
        &mappings(),
        &options(false),
        &path,
        &progress,
    )
    .await
    .unwrap_or_else(|e| panic!("{engine:?} import: {} {:?}", e.message, e.detail));
    assert!(summary.rolled_back, "{engine:?}");
    assert_eq!(
        summary.rejected.len(),
        1,
        "{engine:?}: {:?}",
        summary.rejected
    );
    assert_eq!(summary.rejected[0].line, 6, "the line the bad row is on");
    assert_eq!(
        count(&mut session).await,
        "0",
        "{engine:?} kept rows after stopping"
    );

    // Skip: the rest is kept.
    let progress = ImportProgress::default();
    let summary = run(
        &mut session,
        &target,
        &mappings(),
        &options(true),
        &path,
        &progress,
    )
    .await
    .unwrap_or_else(|e| panic!("{engine:?} import: {} {:?}", e.message, e.detail));
    assert_eq!(summary.inserted, 4, "{engine:?}: {summary:?}");
    assert_eq!(summary.rejected_total, 1);
    assert_eq!(count(&mut session).await, "4");
    let result = session
        .execute(
            "SELECT name, amount, born FROM thabi_import WHERE id = 2",
            1,
        )
        .await
        .unwrap();
    assert_eq!(
        result.rows[0][0],
        Cell::Value("naïve ☕".into()),
        "{engine:?}"
    );
    assert_eq!(result.rows[0][1], Cell::Null, "empty is NULL");
    assert_eq!(result.rows[0][2], Cell::Null);
    let result = session
        .execute("SELECT name FROM thabi_import WHERE id = 1", 1)
        .await
        .unwrap();
    assert_eq!(result.rows[0][0], Cell::Value("O'Hara, Jr".into()));
    let _ = std::fs::remove_file(&path);
}

async fn imports_many_rows(engine: Engine) {
    let mut session = open(engine).await;
    let (db, schema) = place(engine);
    fresh_table(&mut session, engine).await;
    let mut text = String::from("id,name,amount,born,ok\n");
    for i in 0..20_000 {
        text.push_str(&format!("{i},row {i},{}.25,2020-01-01,true\n", i % 1000));
    }
    let path = file(&format!("{engine:?}-many.csv"), &text);
    let target = ImportTarget {
        database: db.into(),
        schema: (!schema.is_empty()).then(|| schema.into()),
        table: "thabi_import".into(),
    };
    let progress = ImportProgress::default();
    let summary = run(
        &mut session,
        &target,
        &mappings(),
        &options(false),
        &path,
        &progress,
    )
    .await
    .unwrap();
    assert_eq!(summary.inserted, 20_000, "{engine:?}");
    assert_eq!(count(&mut session).await, "20000");
    assert!(progress.bytes.load(std::sync::atomic::Ordering::Relaxed) > 0);
    let _ = std::fs::remove_file(&path);
    session.execute("DROP TABLE thabi_import", 0).await.unwrap();
}

macro_rules! engine_tests {
    ($module:ident, $engine:expr) => {
        mod $module {
            use super::*;

            #[tokio::test]
            #[ignore]
            async fn imports_values_and_handles_a_bad_row() {
                super::imports_values_and_handles_a_bad_row($engine).await;
            }

            #[tokio::test]
            #[ignore]
            async fn imports_many_rows() {
                super::imports_many_rows($engine).await;
            }
        }
    };
}

engine_tests!(mysql, Engine::MySql);
engine_tests!(postgres, Engine::Postgres);
engine_tests!(sqlserver, Engine::SqlServer);

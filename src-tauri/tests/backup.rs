//! Backup then restore with the engines' own tools, against real databases.
//! PostgreSQL and MySQL need their client tools on this computer; SQL Server
//! backs up on the server. `#[ignore]`d; run with `npm run test:integration`.

use thabi_studio_lib::backup::{Job, backup, restore};
use thabi_studio_lib::database::{Cell, ConnectParams, Engine, Session, TlsMode};

fn env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} is not set; run npm run test:integration"))
}

fn params(engine: Engine, database: &str) -> ConnectParams {
    let (port, username) = match engine {
        Engine::Postgres => (env("THABI_IT_PG_PORT"), "thabi"),
        // Restoring triggers with binary logging on needs a privileged login.
        Engine::MySql => (env("THABI_IT_MYSQL_PORT"), "root"),
        Engine::SqlServer => (env("THABI_IT_MSSQL_PORT"), "sa"),
    };
    ConnectParams {
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
    }
}

async fn run(session: &mut Session, sql: &str) {
    session
        .execute(sql, 0)
        .await
        .unwrap_or_else(|e| panic!("{sql}: {} {:?}", e.message, e.detail));
}

async fn rows(session: &mut Session) -> Vec<String> {
    let result = session
        .execute("SELECT note FROM thabi_backup ORDER BY id", 100)
        .await
        .unwrap();
    result
        .rows
        .iter()
        .map(|r| match &r[0] {
            Cell::Value(v) => v.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

async fn round_trip(engine: Engine) {
    let database = match engine {
        Engine::SqlServer => "thabi_backup_db",
        _ => "thabi_it",
    };
    if engine == Engine::SqlServer {
        let mut master = Session::connect(&params(engine, "master")).await.unwrap();
        run(&mut master, "IF DB_ID('thabi_backup_db') IS NOT NULL BEGIN ALTER DATABASE thabi_backup_db SET SINGLE_USER WITH ROLLBACK IMMEDIATE; DROP DATABASE thabi_backup_db; END").await;
        run(&mut master, "CREATE DATABASE thabi_backup_db").await;
        master.close().await.unwrap();
    }
    let p = params(engine, database);
    let mut session = Session::connect(&p).await.unwrap();
    let _ = session
        .execute("DROP TABLE IF EXISTS thabi_backup", 0)
        .await;
    let text = if engine == Engine::SqlServer {
        "NVARCHAR(40)"
    } else {
        "VARCHAR(40)"
    };
    run(
        &mut session,
        &format!("CREATE TABLE thabi_backup (id INT PRIMARY KEY, note {text})"),
    )
    .await;
    let n = if engine == Engine::SqlServer { "N" } else { "" };
    run(
        &mut session,
        &format!(
            "INSERT INTO thabi_backup VALUES (1, {n}'naïve ☕'), (2, {n}'O''Hara'), (3, NULL)"
        ),
    )
    .await;
    let before = rows(&mut session).await;

    let path = match engine {
        Engine::SqlServer => std::path::PathBuf::from("/var/opt/mssql/data/thabi_backup_db.bak"),
        Engine::Postgres => std::env::temp_dir().join("thabi-backup-test.dump"),
        Engine::MySql => std::env::temp_dir().join("thabi-backup-test.sql"),
    };
    let job = Job::default();
    let made = backup(&p, database, path.clone(), &job)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "{engine:?} backup: {} {:?}\n{:?}",
                e.message,
                e.detail,
                job.log()
            )
        });
    if engine != Engine::SqlServer {
        assert!(made.bytes.unwrap_or(0) > 0, "{engine:?} wrote nothing");
        let log = job.log().join("\n");
        assert!(
            !log.contains(&env("THABI_IT_PASSWORD")),
            "the password reached the log"
        );
    }

    run(&mut session, "DROP TABLE thabi_backup").await;
    session.close().await.unwrap();

    let job = Job::default();
    restore(&p, database, path.clone(), &job)
        .await
        .unwrap_or_else(|e| {
            panic!(
                "{engine:?} restore: {} {:?}\n{:?}",
                e.message,
                e.detail,
                job.log()
            )
        });
    let mut session = Session::connect(&p).await.unwrap();
    assert_eq!(
        rows(&mut session).await,
        before,
        "{engine:?} restored something else"
    );
    run(&mut session, "DROP TABLE thabi_backup").await;
    session.close().await.unwrap();
    if engine != Engine::SqlServer {
        let _ = std::fs::remove_file(&path);
    }
}

#[tokio::test]
#[ignore]
async fn postgres_backs_up_and_restores() {
    round_trip(Engine::Postgres).await;
}

#[tokio::test]
#[ignore]
async fn mysql_backs_up_and_restores() {
    round_trip(Engine::MySql).await;
}

#[tokio::test]
#[ignore]
async fn sqlserver_backs_up_and_restores_on_the_server() {
    round_trip(Engine::SqlServer).await;
}

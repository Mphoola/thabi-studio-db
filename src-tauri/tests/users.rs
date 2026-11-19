//! Users and permissions against real databases.
//! `#[ignore]`d; run with `npm run test:integration`.

use thabi_studio_lib::database::users::{Scope, UserAction, grants, list_users, user_script};
use thabi_studio_lib::database::{ConnectParams, Engine, Session, TlsMode};

fn env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} is not set; run npm run test:integration"))
}

fn params(engine: Engine, user: &str, password: &str, database: &str) -> ConnectParams {
    let port = match engine {
        Engine::Postgres => env("THABI_IT_PG_PORT"),
        Engine::MySql => env("THABI_IT_MYSQL_PORT"),
        Engine::SqlServer => env("THABI_IT_MSSQL_PORT"),
    };
    ConnectParams {
        engine,
        host: "127.0.0.1".into(),
        port: port.parse().unwrap(),
        database: Some(database.into()),
        username: user.into(),
        password: Some(password.into()),
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

async fn apply(session: &mut Session, engine: Engine, action: &UserAction) {
    let (statements, shown) = user_script(engine, action).unwrap();
    assert!(
        shown.iter().all(|s| !s.contains("Pa'ss")),
        "shown with the password"
    );
    for statement in statements {
        session
            .execute(&statement, 0)
            .await
            .unwrap_or_else(|e| panic!("{engine:?} refused {}: {:?}", e.message, e.detail));
    }
}

async fn manages_a_user(engine: Engine) {
    let (admin, database) = match engine {
        Engine::Postgres => ("thabi", "thabi_it"),
        Engine::MySql => ("root", "thabi_it"),
        Engine::SqlServer => ("sa", "thabi_users_db"),
    };
    let secret = env("THABI_IT_PASSWORD");
    if engine == Engine::SqlServer {
        let mut master = Session::connect(&params(engine, "sa", &secret, "master"))
            .await
            .unwrap();
        let _ = master
            .execute(
                "IF DB_ID('thabi_users_db') IS NULL CREATE DATABASE thabi_users_db",
                0,
            )
            .await;
        let _ = master
            .execute(
                "IF SUSER_ID('thabi_report') IS NOT NULL DROP LOGIN thabi_report",
                0,
            )
            .await;
        master.close().await.unwrap();
    }
    let mut session = Session::connect(&params(engine, admin, &secret, database))
        .await
        .unwrap();
    let host = if engine == Engine::MySql { "%" } else { "" };
    let _ = session
        .execute(
            match engine {
                Engine::MySql => "DROP USER IF EXISTS 'thabi_report'@'%'",
                Engine::Postgres => "DO $$ BEGIN IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'thabi_report') THEN REASSIGN OWNED BY thabi_report TO CURRENT_USER; DROP OWNED BY thabi_report; DROP ROLE thabi_report; END IF; END $$",
                Engine::SqlServer => "DROP USER IF EXISTS thabi_report",
            },
            0,
        )
        .await;
    let _ = session
        .execute("DROP TABLE IF EXISTS thabi_secret", 0)
        .await;
    session
        .execute("CREATE TABLE thabi_secret (id INT)", 0)
        .await
        .unwrap();

    // A password with a quote: it is a literal, not part of the statement.
    let password = "Pa'ss-W0rd-2026!";
    apply(
        &mut session,
        engine,
        &UserAction::CreateUser {
            name: "thabi_report".into(),
            host: host.into(),
            password: password.into(),
            database: database.into(),
        },
    )
    .await;
    let schema = match engine {
        Engine::MySql => None,
        Engine::Postgres => Some("public".to_owned()),
        Engine::SqlServer => Some("dbo".to_owned()),
    };
    apply(
        &mut session,
        engine,
        &UserAction::Grant {
            name: "thabi_report".into(),
            host: host.into(),
            privileges: vec!["SELECT".into()],
            scope: Scope {
                database: database.into(),
                schema: schema.clone(),
                table: Some("thabi_secret".into()),
            },
        },
    )
    .await;
    if engine == Engine::SqlServer {
        apply(
            &mut session,
            engine,
            &UserAction::GrantRole {
                name: "thabi_report".into(),
                host: String::new(),
                role: "db_datareader".into(),
            },
        )
        .await;
    }

    let users = list_users(&mut session, database).await.unwrap();
    let found = users
        .iter()
        .find(|u| u.name == "thabi_report")
        .unwrap_or_else(|| panic!("{engine:?} did not list the user: {users:?}"));
    if engine == Engine::SqlServer {
        assert!(
            found.roles.contains(&"db_datareader".to_owned()),
            "{found:?}"
        );
    }
    let given = grants(&mut session, database, "thabi_report", host)
        .await
        .unwrap();
    assert!(
        given
            .iter()
            .any(|g| g.contains("SELECT") && g.contains("thabi_secret")),
        "{engine:?}: {given:?}"
    );

    // The new login works, and can read what it was given.
    let mut user = Session::connect(&params(engine, "thabi_report", password, database))
        .await
        .unwrap_or_else(|e| panic!("{engine:?} new login: {} {:?}", e.message, e.detail));
    user.execute("SELECT COUNT(*) FROM thabi_secret", 1)
        .await
        .unwrap();
    user.close().await.unwrap();

    apply(
        &mut session,
        engine,
        &UserAction::SetPassword {
            name: "thabi_report".into(),
            host: host.into(),
            password: "Another-Pa'ss-1".into(),
        },
    )
    .await;
    assert!(
        Session::connect(&params(engine, "thabi_report", password, database))
            .await
            .is_err(),
        "{engine:?} kept the old password"
    );

    apply(
        &mut session,
        engine,
        &UserAction::DropUser {
            name: "thabi_report".into(),
            host: host.into(),
        },
    )
    .await;
    let users = list_users(&mut session, database).await.unwrap();
    assert!(
        users.iter().all(|u| u.name != "thabi_report"),
        "{engine:?} kept the user"
    );
    session.execute("DROP TABLE thabi_secret", 0).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn postgres_manages_a_user() {
    manages_a_user(Engine::Postgres).await;
}

#[tokio::test]
#[ignore]
async fn mysql_manages_a_user() {
    manages_a_user(Engine::MySql).await;
}

#[tokio::test]
#[ignore]
async fn sqlserver_manages_a_user() {
    manages_a_user(Engine::SqlServer).await;
}

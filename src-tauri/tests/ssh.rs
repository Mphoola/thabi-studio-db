//! SSH tunnel integration tests. Run with `npm run test:integration`, which
//! starts an SSH bastion next to the databases. Inside the Docker network the
//! databases are reachable by service name (postgres, mysql, sqlserver).

use thabi_studio_lib::{
    database::{ConnectParams, Engine, Session, TlsMode, test_connection},
    ssh::{KnownHost, SshAuth, SshParams},
};

fn env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} is not set; run npm run test:integration"))
}

fn ssh(auth: SshAuth, known_keys: Vec<KnownHost>) -> SshParams {
    SshParams {
        host: "127.0.0.1".into(),
        port: env("THABI_IT_SSH_PORT").parse().unwrap(),
        username: "thabi".into(),
        auth,
        known_keys,
        proxy_jump: None,
        keepalive: None,
    }
}

fn password() -> SshAuth {
    SshAuth::Password(env("THABI_IT_PASSWORD"))
}

fn key(name: &str, passphrase: Option<String>) -> SshAuth {
    SshAuth::KeyFile {
        path: std::path::Path::new(&env("THABI_IT_SSH_KEY_DIR")).join(name),
        passphrase,
    }
}

fn database(engine: Engine, ssh: SshParams) -> ConnectParams {
    let (host, port, username, database, tls) = match engine {
        Engine::Postgres => ("postgres", 5432, "thabi", "thabi_it", TlsMode::Off),
        Engine::MySql => ("mysql", 3306, "thabi", "thabi_it", TlsMode::EncryptOnly),
        Engine::SqlServer => ("sqlserver", 1433, "sa", "master", TlsMode::EncryptOnly),
    };
    ConnectParams {
        engine,
        host: host.into(),
        port,
        database: Some(database.into()),
        username: username.into(),
        password: Some(env("THABI_IT_PASSWORD")),
        tls,
        ca_path: None,
        client_cert: None,
        client_key: None,
        ssh: Some(ssh),
        connect_to: None,
        read_only: false,
        startup_commands: Vec::new(),
    }
}

/// Connects once without trusting anything, checks that the unknown host
/// key is reported with its fingerprint, and returns that key trusted.
async fn trusted_host_key() -> Vec<KnownHost> {
    let error = Session::connect(&database(Engine::Postgres, ssh(password(), vec![])))
        .await
        .err()
        .expect("an unknown host key must not be accepted");
    assert_eq!(
        error.kind, "ssh_host_unknown",
        "{} {:?}",
        error.message, error.detail
    );
    let data = error.data.expect("host key data");
    assert!(data["fingerprint"].as_str().unwrap().starts_with("SHA256:"));
    let key = russh::keys::PublicKey::from_openssh(data["publicKey"].as_str().unwrap()).unwrap();
    vec![KnownHost {
        key,
        from_openssh: false,
    }]
}

#[tokio::test]
#[ignore]
async fn unknown_host_keys_need_confirmation_and_then_connect() {
    let known = trusted_host_key().await;
    let result = test_connection(&database(Engine::Postgres, ssh(password(), known)))
        .await
        .unwrap();
    assert!(result.server.version.starts_with("PostgreSQL"));
}

#[tokio::test]
#[ignore]
async fn a_changed_host_key_is_refused() {
    trusted_host_key().await;
    let other = russh::keys::PublicKey::from_openssh(
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFUCl4YJ7IyIa2yEUkDjnPsOJeBcMukxWYMKAuDcbw27 other",
    )
    .unwrap();
    let error = Session::connect(&database(
        Engine::Postgres,
        ssh(
            password(),
            vec![KnownHost {
                key: other,
                from_openssh: false,
            }],
        ),
    ))
    .await
    .err()
    .unwrap();
    assert_eq!(error.kind, "ssh_host_changed");
}

#[tokio::test]
#[ignore]
async fn every_engine_works_through_the_tunnel() {
    let known = trusted_host_key().await;
    for (engine, auth) in [
        (Engine::Postgres, password()),
        (Engine::MySql, key("id_plain", None)),
        (
            Engine::SqlServer,
            key("id_locked", Some(env("THABI_IT_PASSWORD"))),
        ),
    ] {
        let mut session = Session::connect(&database(engine, ssh(auth, known.clone())))
            .await
            .unwrap_or_else(|e| panic!("{engine:?}: {} {:?}", e.message, e.detail));
        let result = session.execute("SELECT 1", 10).await.unwrap();
        assert_eq!(result.rows.len(), 1, "{engine:?}");
        session.close().await.unwrap();
    }
}

#[tokio::test]
#[ignore]
async fn closing_the_session_closes_the_tunnel() {
    let known = trusted_host_key().await;
    let session = Session::connect(&database(Engine::Postgres, ssh(password(), known)))
        .await
        .unwrap();
    let local = session.tunnel_addr().expect("tunnel address");
    assert!(tokio::net::TcpStream::connect(local).await.is_ok());
    session.close().await.unwrap();
    assert!(tokio::net::TcpStream::connect(local).await.is_err());
}

/// One ProxyJump hop, as `~/.ssh/config` allows it: the first bastion is
/// reached directly, and it forwards on to the second, which carries the
/// database connection. Both bastions are the same image; the jump reaches
/// the target by its Compose service name, so nothing extra is published.
#[tokio::test]
#[ignore]
async fn a_proxy_jump_config_reaches_the_server_through_a_bastion() {
    // The jump host: reached directly from the test, on its own port.
    let mut jump = ssh(password(), vec![]);
    jump.port = env("THABI_IT_SSH_JUMP_PORT").parse().unwrap();
    let jump_keys = {
        let error = thabi_studio_lib::ssh::open_tunnel(&jump, "postgres", 5432)
            .await
            .err()
            .expect("an unknown jump host key must not be accepted");
        assert_eq!(error.kind, "ssh_host_unknown", "{error:?}");
        let data = error.data.expect("host key data");
        vec![KnownHost {
            key: russh::keys::PublicKey::from_openssh(data["publicKey"].as_str().unwrap()).unwrap(),
            from_openssh: false,
        }]
    };
    jump.known_keys = jump_keys;

    // The target: only the jump host can see it, by its service name.
    let mut target = ssh(password(), trusted_host_key().await);
    target.host = "sshd".into();
    target.port = 22;
    target.proxy_jump = Some(Box::new(jump));

    let mut session = Session::connect(&database(Engine::Postgres, target))
        .await
        .unwrap_or_else(|e| panic!("{} {:?}", e.message, e.detail));
    let result = session.execute("SELECT 1", 10).await.unwrap();
    assert_eq!(result.rows.len(), 1);
    session.close().await.unwrap();
}

/// Closing the connection must take the jump tunnel with it.
#[tokio::test]
#[ignore]
async fn closing_a_jumped_tunnel_closes_the_jump() {
    let mut jump = ssh(password(), vec![]);
    jump.port = env("THABI_IT_SSH_JUMP_PORT").parse().unwrap();
    jump.known_keys = vec![];

    let mut target = ssh(password(), trusted_host_key().await);
    target.host = "sshd".into();
    target.port = 22;
    // Trust the jump by connecting to it once through a direct tunnel.
    let error = thabi_studio_lib::ssh::open_tunnel(&jump, "postgres", 5432)
        .await
        .err()
        .expect("first jump connection needs confirmation");
    jump.known_keys = vec![KnownHost {
        key: russh::keys::PublicKey::from_openssh(
            error.data.expect("host key data")["publicKey"]
                .as_str()
                .unwrap(),
        )
        .unwrap(),
        from_openssh: false,
    }];
    target.proxy_jump = Some(Box::new(jump));

    let session = Session::connect(&database(Engine::Postgres, target))
        .await
        .unwrap_or_else(|e| panic!("{} {:?}", e.message, e.detail));
    let local = session.tunnel_addr().expect("tunnel address");
    assert!(tokio::net::TcpStream::connect(local).await.is_ok());
    session.close().await.unwrap();
    assert!(tokio::net::TcpStream::connect(local).await.is_err());
}

#[tokio::test]
#[ignore]
async fn ssh_failures_have_their_own_messages() {
    let known = trusted_host_key().await;

    let error = Session::connect(&database(
        Engine::Postgres,
        ssh(SshAuth::Password("wrong".into()), known.clone()),
    ))
    .await
    .err()
    .unwrap();
    assert_eq!(error.kind, "ssh_authentication");

    let error = Session::connect(&database(
        Engine::Postgres,
        ssh(key("id_locked", None), known.clone()),
    ))
    .await
    .err()
    .unwrap();
    assert_eq!(error.kind, "ssh_key");

    let mut unreachable = database(Engine::Postgres, ssh(password(), known.clone()));
    unreachable.host = "no-such-database-host".into();
    let error = Session::connect(&unreachable).await.err().unwrap();
    assert_eq!(
        error.kind, "ssh_forward",
        "{} {:?}",
        error.message, error.detail
    );

    let mut refused = ssh(password(), known);
    refused.port = 1;
    let error = Session::connect(&database(Engine::Postgres, refused))
        .await
        .err()
        .unwrap();
    assert_eq!(error.kind, "ssh_connection");
}

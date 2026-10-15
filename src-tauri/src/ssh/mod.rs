//! SSH tunnels for database connections.
//!
//! A tunnel is an SSH session plus a listener on 127.0.0.1 with a random
//! port. Each TCP connection to that port becomes a `direct-tcpip` channel
//! to the database host as seen from the SSH server. The listener only
//! accepts local connections and lives exactly as long as the database
//! session that owns it.

mod host_keys;
mod legacy_pem;

use crate::errors::AppError;
use russh::{
    Disconnect,
    client::{self, Handle},
    keys::{HashAlg, PrivateKeyWithHashAlg, PublicKey, PublicKeyOrCertificate},
};
use serde_json::json;
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{net::TcpListener, task::JoinHandle};

pub use host_keys::{KnownHost, forget_host_keys, known_host_keys, trust_host_key};

const SSH_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const KEEPALIVE: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub enum SshAuth {
    Password(String),
    KeyFile {
        path: PathBuf,
        passphrase: Option<String>,
    },
    /// Keys held by the user's ssh-agent (SSH_AUTH_SOCK).
    Agent,
}

#[derive(Clone)]
pub struct SshParams {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
    /// Host keys already trusted for this server. Empty means "unknown".
    pub known_keys: Vec<KnownHost>,
}

impl std::fmt::Debug for SshParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let auth = match &self.auth {
            SshAuth::Password(_) => "password",
            SshAuth::KeyFile { .. } => "key file",
            SshAuth::Agent => "agent",
        };
        f.debug_struct("SshParams")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("auth", &auth)
            .field("known_keys", &self.known_keys.len())
            .finish()
    }
}

/// What the server presented, kept so a failed check can explain itself.
#[derive(Default)]
struct HostKeyCheck {
    presented: Option<PublicKey>,
    changed: bool,
}

struct ClientHandler {
    known: Vec<KnownHost>,
    check: Arc<Mutex<HostKeyCheck>>,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let PublicKeyOrCertificate::PublicKey { key, .. } = server_key else {
            // Host certificates aren't supported yet; never trust them blindly.
            return Ok(false);
        };
        let mut check = self.check.lock().expect("host key check lock");
        check.presented = Some(key.clone());
        let same_algorithm: Vec<_> = self
            .known
            .iter()
            .filter(|k| k.key.algorithm() == key.algorithm())
            .collect();
        if same_algorithm
            .iter()
            .any(|k| k.key.key_data() == key.key_data())
        {
            return Ok(true);
        }
        // A different key of the same type for this host means it changed.
        check.changed = !same_algorithm.is_empty();
        Ok(false)
    }
}

pub struct Tunnel {
    local_addr: SocketAddr,
    session: Arc<Handle<ClientHandler>>,
    accept_task: JoinHandle<()>,
}

impl Tunnel {
    /// Local address the database driver should connect to.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn close(self) {
        self.accept_task.abort();
        let _ = self
            .session
            .disconnect(
                Disconnect::ByApplication,
                "Thabi Studio closed the connection",
                "en",
            )
            .await;
    }
}

fn host_key_data(host: &str, port: u16, key: &PublicKey) -> serde_json::Value {
    json!({
        "host": host,
        "port": port,
        "algorithm": key.algorithm().to_string(),
        "fingerprint": key.fingerprint(HashAlg::Sha256).to_string(),
        "publicKey": key.to_openssh().unwrap_or_default(),
    })
}

fn ssh_io_error(params: &SshParams, error: &std::io::Error) -> AppError {
    let message = match error.kind() {
        std::io::ErrorKind::ConnectionRefused => format!(
            "The SSH server refused the connection on port {}. Check the SSH host and port.",
            params.port
        ),
        _ => format!(
            "Couldn't reach the SSH server {}:{}.",
            params.host, params.port
        ),
    };
    AppError::new("ssh_connection", message).with_detail(Some(error.to_string()))
}

/// Opens an SSH session, authenticates and starts forwarding a local port
/// to `target_host:target_port`.
pub async fn open_tunnel(
    params: &SshParams,
    target_host: &str,
    target_port: u16,
) -> Result<Tunnel, AppError> {
    let config = Arc::new(client::Config {
        keepalive_interval: Some(KEEPALIVE),
        keepalive_max: 3,
        nodelay: true,
        ..Default::default()
    });
    let check = Arc::new(Mutex::new(HostKeyCheck::default()));
    let handler = ClientHandler {
        known: params.known_keys.clone(),
        check: check.clone(),
    };

    let address = crate::database::error::check_host_resolves(&params.host, params.port)
        .await
        .map_err(|_| {
            AppError::new(
                "ssh_host_not_found",
                format!(
                    "Couldn't find the SSH host “{}”. Check the host name.",
                    params.host
                ),
            )
        })
        .map(|_| (params.host.clone(), params.port))?;

    let connected = tokio::time::timeout(SSH_CONNECT_TIMEOUT, async {
        let stream = tokio::net::TcpStream::connect(&address)
            .await
            .map_err(|e| ssh_io_error(params, &e))?;
        stream.set_nodelay(true).ok();
        client::connect_stream(config, stream, handler)
            .await
            .map_err(|e: russh::Error| {
                AppError::new(
                    "ssh_connection",
                    format!("The SSH connection to {} failed.", params.host),
                )
                .with_detail(Some(e.to_string()))
            })
    })
    .await
    .map_err(|_| {
        AppError::new(
            "ssh_timeout",
            format!(
                "The SSH server {}:{} didn't respond within 10 seconds.",
                params.host, params.port
            ),
        )
    })?;

    let mut session = match connected {
        Ok(session) => session,
        Err(error) => {
            // Explain a rejected host key instead of a generic failure.
            let check = check.lock().expect("host key check lock");
            if let Some(key) = &check.presented {
                let data = host_key_data(&params.host, params.port, key);
                return Err(if check.changed {
                    AppError::new(
                        "ssh_host_changed",
                        "The SSH server's host key has changed. Someone may be intercepting the connection. If the server was rebuilt, forget the saved key and connect again.",
                    )
                    .with_detail(Some(format!("New key: {}", data["fingerprint"].as_str().unwrap_or_default())))
                    .with_data(data)
                } else {
                    AppError::new(
                        "ssh_host_unknown",
                        "Thabi Studio hasn't connected to this SSH server before. Check its fingerprint, then trust it to continue.",
                    )
                    .with_detail(data["fingerprint"].as_str().map(str::to_owned))
                    .with_data(data)
                });
            }
            return Err(error);
        }
    };

    authenticate(&mut session, params).await?;

    let listener = TcpListener::bind(("127.0.0.1", 0)).await.map_err(|e| {
        AppError::new("ssh_forward", "Couldn't open a local port for the tunnel.")
            .with_detail(Some(e.to_string()))
    })?;
    let local_addr = listener
        .local_addr()
        .map_err(|_| AppError::background_task())?;

    // Open one channel now so a wrong database host fails here, clearly.
    let probe = session
        .channel_open_direct_tcpip(target_host, target_port.into(), "127.0.0.1", 0)
        .await
        .map_err(|e| forward_error(target_host, target_port, e))?;
    let _ = probe.close().await;

    let session = Arc::new(session);
    let session_for_task = session.clone();
    let target = (target_host.to_owned(), target_port);
    let accept_task = tokio::spawn(async move {
        while let Ok((mut socket, peer)) = listener.accept().await {
            let session = session_for_task.clone();
            let (host, port) = target.clone();
            tokio::spawn(async move {
                let Ok(channel) = session
                    .channel_open_direct_tcpip(
                        host,
                        port.into(),
                        peer.ip().to_string(),
                        peer.port().into(),
                    )
                    .await
                else {
                    return;
                };
                let mut stream = channel.into_stream();
                let _ = tokio::io::copy_bidirectional(&mut socket, &mut stream).await;
            });
        }
    });

    Ok(Tunnel {
        local_addr,
        session,
        accept_task,
    })
}

fn forward_error(host: &str, port: u16, error: russh::Error) -> AppError {
    AppError::new(
        "ssh_forward",
        format!(
            "The SSH server couldn't reach {host}:{port}. Check the database host and port as seen from the SSH server."
        ),
    )
    .with_detail(Some(error.to_string()))
}

const NEEDS_PASSPHRASE: &str =
    "This SSH key is protected by a passphrase. Enter it in the connection settings.";

/// Reads a private key file, including old OpenSSL-encrypted PEM keys.
fn load_key(
    path: &std::path::Path,
    passphrase: Option<&str>,
) -> Result<russh::keys::PrivateKey, AppError> {
    use legacy_pem::LegacyPemError;
    let text = std::fs::read_to_string(path).map_err(|e| {
        AppError::new(
            "ssh_key",
            "The SSH key file couldn't be opened. Check that it still exists.",
        )
        .with_detail(Some(e.to_string()))
    })?;
    let text = match legacy_pem::decrypt(&text, passphrase) {
        Ok(Some(decrypted)) => decrypted,
        Ok(None) => text,
        Err(LegacyPemError::NeedsPassphrase) => {
            return Err(AppError::new("ssh_key", NEEDS_PASSPHRASE));
        }
        Err(LegacyPemError::WrongPassphrase) => {
            return Err(AppError::new(
                "ssh_key",
                "The passphrase didn't unlock this SSH key. Check it and try again.",
            ));
        }
        Err(LegacyPemError::Unsupported(cipher)) => {
            return Err(AppError::new(
                "ssh_key",
                format!(
                    "This SSH key is encrypted with {cipher}, which Thabi Studio can't read. Convert it with ssh-keygen -p -f <key>."
                ),
            ));
        }
        Err(LegacyPemError::Malformed) => text,
    };
    russh::keys::decode_secret_key(&text, passphrase).map_err(|e| {
        let message = match e {
            russh::keys::Error::KeyIsEncrypted => NEEDS_PASSPHRASE,
            _ => "The SSH key file couldn't be read. Check the file and passphrase.",
        };
        AppError::new("ssh_key", message).with_detail(Some(e.to_string()))
    })
}

async fn authenticate(
    session: &mut Handle<ClientHandler>,
    params: &SshParams,
) -> Result<(), AppError> {
    let rejected = || {
        AppError::new(
            "ssh_authentication",
            "The SSH server rejected the user name or credentials.",
        )
    };
    let failed = |e: russh::Error| {
        AppError::new(
            "ssh_connection",
            "SSH authentication couldn't be completed.",
        )
        .with_detail(Some(e.to_string()))
    };
    let ok = match &params.auth {
        SshAuth::Password(password) => {
            let result = session
                .authenticate_password(&params.username, password)
                .await
                .map_err(failed)?;
            // Many servers (PAM, e.g. default Ubuntu and macOS sshd) turn off
            // plain "password" and ask for it through keyboard-interactive.
            let offers_interactive = matches!(
                &result,
                russh::client::AuthResult::Failure { remaining_methods, .. }
                    if remaining_methods.contains(&russh::MethodKind::KeyboardInteractive)
            );
            if result.success() {
                true
            } else if offers_interactive {
                authenticate_interactive(session, &params.username, password)
                    .await
                    .map_err(failed)?
            } else {
                false
            }
        }
        SshAuth::KeyFile { path, passphrase } => {
            let key = load_key(path, passphrase.as_deref())?;
            let hash = session
                .best_supported_rsa_hash()
                .await
                .map_err(failed)?
                .flatten();
            session
                .authenticate_publickey(
                    &params.username,
                    PrivateKeyWithHashAlg::new(Arc::new(key), hash),
                )
                .await
                .map_err(failed)?
                .success()
        }
        SshAuth::Agent => authenticate_with_agent(session, params).await?,
    };
    if ok { Ok(()) } else { Err(rejected()) }
}

/// Answers keyboard-interactive prompts with the saved password. Only
/// password-style prompts are answered; a second factor (an OTP prompt after
/// the password) isn't supported and fails as a rejected login.
async fn authenticate_interactive(
    session: &mut Handle<ClientHandler>,
    username: &str,
    password: &str,
) -> Result<bool, russh::Error> {
    use russh::client::KeyboardInteractiveAuthResponse as Response;
    let mut response = session
        .authenticate_keyboard_interactive_start(username, None::<String>)
        .await?;
    let mut answered_password = false;
    loop {
        match response {
            Response::Success => return Ok(true),
            Response::Failure { .. } => return Ok(false),
            Response::InfoRequest { prompts, .. } => {
                if prompts.iter().any(|p| !p.echo) {
                    if answered_password {
                        return Ok(false);
                    }
                    answered_password = true;
                }
                let answers = prompts
                    .iter()
                    .map(|p| {
                        if p.echo {
                            String::new()
                        } else {
                            password.to_owned()
                        }
                    })
                    .collect();
                response = session
                    .authenticate_keyboard_interactive_respond(answers)
                    .await?;
            }
        }
    }
}

async fn authenticate_with_agent(
    session: &mut Handle<ClientHandler>,
    params: &SshParams,
) -> Result<bool, AppError> {
    let unavailable = || {
        AppError::new(
            "ssh_agent",
            "Couldn't reach your SSH agent. Check that it's running and has your key (ssh-add -l).",
        )
    };
    use russh::keys::agent::client::AgentClient;
    #[cfg(unix)]
    let mut agent = AgentClient::connect_env()
        .await
        .map_err(|_| unavailable())?
        .dynamic();
    // Windows: the OpenSSH agent service's named pipe, or Pageant.
    #[cfg(windows)]
    let mut agent = match AgentClient::connect_named_pipe(r"\\.\pipe\openssh-ssh-agent").await {
        Ok(agent) => agent.dynamic(),
        Err(_) => AgentClient::connect_pageant()
            .await
            .map_err(|_| unavailable())?
            .dynamic(),
    };
    let identities = agent
        .request_identities()
        .await
        .map_err(|_| unavailable())?;
    let hash = session
        .best_supported_rsa_hash()
        .await
        .ok()
        .flatten()
        .flatten();
    for identity in identities {
        let russh::keys::agent::AgentIdentity::PublicKey { key, .. } = identity else {
            continue;
        };
        if let Ok(result) = session
            .authenticate_publickey_with(&params.username, key, hash, &mut agent)
            .await
            && result.success()
        {
            return Ok(true);
        }
    }
    Ok(false)
}

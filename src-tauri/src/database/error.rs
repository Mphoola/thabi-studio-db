//! Turns driver errors into messages that say what went wrong and what to
//! try. The server's own text is kept as `detail`, sanitised first.

use super::{Engine, TlsMode};
use crate::errors::AppError;
use std::io;

const MAX_DETAIL_CHARS: usize = 400;

/// What we know about the attempt, for writing specific messages.
#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub engine: Engine,
    pub host: &'a str,
    pub port: u16,
    pub database: Option<&'a str>,
    pub password: Option<&'a str>,
    pub tls: TlsMode,
}

/// Removes the password if a server echoes it back, flattens whitespace and
/// caps the length. Applied to every server message we pass on.
pub fn sanitise(text: &str, password: Option<&str>) -> String {
    let mut text = text.to_owned();
    if let Some(password) = password.filter(|p| p.len() >= 3) {
        text = text.replace(password, "••••");
    }
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > MAX_DETAIL_CHARS {
        let cut: String = flat.chars().take(MAX_DETAIL_CHARS).collect();
        format!("{cut}…")
    } else {
        flat
    }
}

pub async fn check_host_resolves(host: &str, port: u16) -> Result<(), AppError> {
    if let Ok(mut addresses) = tokio::net::lookup_host((host, port)).await
        && addresses.next().is_some()
    {
        return Ok(());
    }
    Err(AppError::new(
        "host_not_found",
        format!("Couldn't find the host “{host}”. Check the host name, or your VPN or DNS."),
    ))
}

/// Context for errors on an open session, where only the address matters.
pub fn session_context(engine: Engine, host: &str, port: u16) -> Context<'_> {
    Context {
        engine,
        host,
        port,
        database: None,
        password: None,
        tls: TlsMode::Verify,
    }
}

pub fn timed_out(host: &str, port: u16) -> AppError {
    AppError::new(
        "timeout",
        format!(
            "The server at {host}:{port} didn't respond within 10 seconds. Check the host and port, and any firewall or VPN."
        ),
    )
}

/// Some drivers report certificate failures as plain I/O errors.
fn looks_like_tls(text: &str) -> bool {
    let lower = text.to_lowercase();
    ["certificate", "tls", "ssl", "handshake"]
        .iter()
        .any(|word| lower.contains(word))
}

pub fn from_io(kind: io::ErrorKind, message: &str, ctx: Context<'_>) -> AppError {
    if !matches!(kind, io::ErrorKind::ConnectionRefused) && looks_like_tls(message) {
        return tls(ctx, message);
    }
    let detail = Some(sanitise(message, ctx.password));
    match kind {
        io::ErrorKind::ConnectionRefused => AppError::new(
            "connection_refused",
            format!(
                "The server refused the connection on port {}. Check the host and port, and that the database is running.",
                ctx.port
            ),
        ),
        io::ErrorKind::HostUnreachable | io::ErrorKind::NetworkUnreachable => AppError::new(
            "unreachable",
            format!("Couldn't reach {}. Check your network or VPN.", ctx.host),
        ),
        io::ErrorKind::TimedOut => timed_out(ctx.host, ctx.port),
        io::ErrorKind::ConnectionReset
        | io::ErrorKind::ConnectionAborted
        | io::ErrorKind::UnexpectedEof
        | io::ErrorKind::BrokenPipe => closed_by_server(ctx).with_detail(detail),
        _ => AppError::new(
            "connection",
            format!("Couldn't connect to {}:{}.", ctx.host, ctx.port),
        )
        .with_detail(detail),
    }
}

/// The server accepted the connection and closed it without speaking.
///
/// The driver reads a greeting as soon as it connects, so this is the earliest
/// possible failure: whatever closed it did so before saying which server it
/// was. There is no evidence here about *why*, so this says what is worth
/// checking rather than picking one cause and presenting it as the answer.
///
/// Listed, most common first: the server is out of connection slots and drops
/// new arrivals; the port belongs to a different kind of database, whose
/// handshake ours cannot read; the server requires encryption; or it restarted
/// mid-connect. Naming the port and engine is the part that actually helps,
/// because the usual cause is a profile pointing somewhere unintended.
fn closed_by_server(ctx: Context<'_>) -> AppError {
    let mut causes: Vec<String> = Vec::new();
    causes.push("the server may have no connection slots left".to_owned());
    causes.push(format!(
        "port {} may belong to a different kind of database than {}",
        ctx.port,
        ctx.engine.label()
    ));
    if ctx.tls == TlsMode::Off {
        causes.push(
            "the server may require encryption — try a TLS setting other than Off".to_owned(),
        );
    }
    causes.push("the server may have restarted".to_owned());
    AppError::new(
        "connection_closed",
        format!(
            "{} closed the connection to {}:{} before it identified itself. Worth checking: {}.",
            ctx.engine.label(),
            ctx.host,
            ctx.port,
            causes.join("; ")
        ),
    )
}

pub fn authentication(detail: Option<String>, code: Option<String>) -> AppError {
    AppError::new(
        "authentication",
        "The server rejected the user name or password.",
    )
    .with_detail(detail)
    .with_code(code)
}

pub fn unknown_database(
    ctx: Context<'_>,
    detail: Option<String>,
    code: Option<String>,
) -> AppError {
    let name = ctx.database.unwrap_or_default();
    AppError::new(
        "unknown_database",
        format!("The database “{name}” doesn't exist, or this user can't open it."),
    )
    .with_detail(detail)
    .with_code(code)
}

pub fn tls(ctx: Context<'_>, detail: &str) -> AppError {
    let lower = detail.to_lowercase();
    if lower.contains("not support") || lower.contains("doesn't support") {
        return AppError::new(
            "tls_unsupported",
            "The server doesn't accept encrypted connections. If you trust this network, set TLS to Off.",
        )
        .with_detail(Some(sanitise(detail, ctx.password)));
    }
    let message = match ctx.tls {
        TlsMode::Verify => {
            "The server's certificate couldn't be verified. Check the host name, choose its CA certificate, or, for a server you trust, use Encrypt Without Verifying."
        }
        _ => "The encrypted connection couldn't be set up.",
    };
    AppError::new("tls", message).with_detail(Some(sanitise(detail, ctx.password)))
}

pub fn permission_denied(ctx: Context<'_>, detail: &str, code: Option<String>) -> AppError {
    AppError::new(
        "permission_denied",
        "This user doesn't have permission to read that.",
    )
    .with_detail(Some(sanitise(detail, ctx.password)))
    .with_code(code)
}

/// The session can't be used any more (server gone, network dropped).
pub fn connection_lost() -> AppError {
    AppError::new(
        "connection_lost",
        "The connection to the database was lost. Reconnect to continue.",
    )
}

pub fn database_not_open(database: &str, current: &str) -> AppError {
    AppError::new(
        "database_not_open",
        format!("This connection is using “{current}”. Switch to “{database}” to browse it."),
    )
}

pub fn server(ctx: Context<'_>, detail: &str, code: Option<String>) -> AppError {
    AppError::new("server", "The database returned an error.")
        .with_detail(Some(sanitise(detail, ctx.password)))
        .with_code(code)
}

/// MySQL and PostgreSQL both come through SQLx.
pub fn from_sqlx(error: sqlx::Error, ctx: Context<'_>) -> AppError {
    match error {
        sqlx::Error::Io(io) => from_io(io.kind(), &io.to_string(), ctx),
        sqlx::Error::Tls(tls_error) => tls(ctx, &tls_error.to_string()),
        sqlx::Error::Database(db) => {
            let message = sanitise(db.message(), ctx.password);
            let (auth, unknown_db, denied, code) = match ctx.engine {
                Engine::MySql => {
                    let number = db
                        .try_downcast_ref::<sqlx::mysql::MySqlDatabaseError>()
                        .map(|e| e.number());
                    (
                        matches!(number, Some(1045 | 1698)),
                        matches!(number, Some(1049 | 1044)),
                        matches!(number, Some(1142 | 1143)),
                        number.map(|n| n.to_string()),
                    )
                }
                _ => {
                    let code = db.code().map(|c| c.into_owned());
                    (
                        matches!(code.as_deref(), Some("28P01" | "28000")),
                        code.as_deref() == Some("3D000"),
                        code.as_deref() == Some("42501"),
                        code,
                    )
                }
            };
            if denied {
                permission_denied(ctx, &message, code)
            } else if auth {
                authentication(Some(message), code)
            } else if unknown_db {
                unknown_database(ctx, Some(message), code)
            } else {
                server(ctx, &message, code)
            }
        }
        other => {
            let text = other.to_string();
            if looks_like_tls(&text) {
                tls(ctx, &text)
            } else {
                AppError::new(
                    "connection",
                    format!("Couldn't connect to {}:{}.", ctx.host, ctx.port),
                )
                .with_detail(Some(sanitise(&text, ctx.password)))
            }
        }
    }
}

pub fn from_tiberius(error: tiberius::error::Error, ctx: Context<'_>) -> AppError {
    use tiberius::error::{Error, IoErrorKind};
    match error {
        Error::Io { kind, message } => {
            let kind = match kind {
                IoErrorKind::ConnectionRefused => io::ErrorKind::ConnectionRefused,
                IoErrorKind::ConnectionReset => io::ErrorKind::ConnectionReset,
                IoErrorKind::ConnectionAborted => io::ErrorKind::ConnectionAborted,
                IoErrorKind::TimedOut => io::ErrorKind::TimedOut,
                IoErrorKind::UnexpectedEof => io::ErrorKind::UnexpectedEof,
                IoErrorKind::BrokenPipe => io::ErrorKind::BrokenPipe,
                _ => io::ErrorKind::Other,
            };
            from_io(kind, &message, ctx)
        }
        Error::Tls(message) => tls(ctx, &message),
        Error::Server(token) => {
            let code = token.code();
            let message = sanitise(token.message(), ctx.password);
            match code {
                18456 => authentication(Some(message), Some(code.to_string())),
                4060 => unknown_database(ctx, Some(message), Some(code.to_string())),
                229 | 262 | 916 => permission_denied(ctx, &message, Some(code.to_string())),
                _ => server(ctx, &message, Some(code.to_string())),
            }
        }
        Error::Routing { host, port } => AppError::new(
            "routing",
            format!(
                "The server redirected the connection to {host}:{port}. Connect to that address instead."
            ),
        ),
        other => server(ctx, &other.to_string(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context<'static> {
        Context {
            engine: Engine::Postgres,
            host: "db.internal",
            port: 5432,
            database: Some("orders"),
            password: Some("s3cret-pass"),
            tls: TlsMode::Verify,
        }
    }

    #[test]
    fn sanitise_removes_the_password_and_limits_length() {
        let text = format!("login failed for s3cret-pass {}", "x".repeat(1000));
        let clean = sanitise(&text, Some("s3cret-pass"));
        assert!(!clean.contains("s3cret-pass"));
        assert!(clean.chars().count() <= MAX_DETAIL_CHARS + 1);
    }

    #[test]
    fn refused_and_unreachable_have_different_messages() {
        let refused = from_io(io::ErrorKind::ConnectionRefused, "refused", ctx());
        let unreachable = from_io(io::ErrorKind::HostUnreachable, "no route", ctx());
        assert_eq!(refused.kind, "connection_refused");
        assert!(refused.message.contains("port 5432"));
        assert_eq!(unreachable.kind, "unreachable");
        assert!(unreachable.message.contains("db.internal"));
    }

    #[test]
    fn tls_without_server_support_suggests_turning_tls_off() {
        let error = tls(ctx(), "server does not support TLS");
        assert_eq!(error.kind, "tls_unsupported");
    }

    #[test]
    fn certificate_failure_explains_the_options() {
        let error = tls(ctx(), "invalid peer certificate: UnknownIssuer");
        assert_eq!(error.kind, "tls");
        assert!(error.message.contains("CA certificate"));
        assert!(error.detail.unwrap().contains("UnknownIssuer"));
    }

    #[test]
    fn sql_server_codes_map_to_specific_errors() {
        let auth = authentication(None, Some("18456".into()));
        assert_eq!(auth.kind, "authentication");
        let db = unknown_database(ctx(), None, None);
        assert!(db.message.contains("“orders”"));
    }

    #[test]
    fn certificate_errors_reported_as_io_are_tls_errors() {
        let error = from_io(
            io::ErrorKind::InvalidData,
            "invalid peer certificate: UnknownIssuer",
            ctx(),
        );
        assert_eq!(error.kind, "tls");
    }

    #[test]
    /// With TLS off, encryption is still worth raising — but as one cause
    /// among several, not as the answer.
    fn closed_connection_with_tls_off_still_mentions_encryption() {
        let mut context = ctx();
        context.tls = TlsMode::Off;
        let error = from_io(io::ErrorKind::ConnectionReset, "reset", context);
        assert_eq!(error.kind, "connection_closed");
        assert!(error.message.contains("encryption"), "{}", error.message);
    }

    /// A guessed cause costs more than no cause: it sends people to one place
    /// and away from the others. So the message must not single out TLS when
    /// TLS was never the problem, and must name where it actually connected.
    #[test]
    fn closed_connection_does_not_blame_tls_when_tls_was_on() {
        let mut context = ctx();
        context.tls = TlsMode::EncryptOnly;
        let error = from_io(io::ErrorKind::ConnectionReset, "reset", context);
        assert!(
            !error.message.contains("encryption"),
            "blamed encryption without evidence: {}",
            error.message
        );
        assert!(error.message.contains(context.host), "{}", error.message);
    }

    #[test]
    fn closed_connection_names_the_port_and_engine() {
        let context = ctx();
        let error = from_io(io::ErrorKind::UnexpectedEof, "eof", context);
        assert!(
            error.message.contains(&context.port.to_string()),
            "{}",
            error.message
        );
        assert!(
            error.message.contains(context.engine.label()),
            "{}",
            error.message
        );
        // The likeliest cause on a local machine goes first.
        assert!(
            error.message.contains("connection slots"),
            "{}",
            error.message
        );
    }
}

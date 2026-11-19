//! Finding the native tools and building their command lines.
//!
//! The password never goes on a command line, where any process list would
//! show it: PostgreSQL's tools read it from `PGPASSWORD`, MySQL's from an
//! option file only the user can read, deleted as soon as the tool exits.

use crate::database::{ConnectParams, Engine, TlsMode};
use crate::errors::AppError;
use std::path::{Path, PathBuf};

/// Every directory a tool might be in. A macOS app doesn't see the shell's
/// PATH, and people often have several versions installed side by side.
fn search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    let mut children = |root: &str, suffix: &str| {
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                dirs.push(entry.path().join(suffix));
            }
        }
    };
    #[cfg(target_os = "macos")]
    {
        children("/opt/homebrew/opt", "bin");
        children("/usr/local/opt", "bin");
        children("/Library/PostgreSQL", "bin");
        children("/Applications/Postgres.app/Contents/Versions", "bin");
    }
    #[cfg(target_os = "windows")]
    {
        children("C:\\Program Files\\PostgreSQL", "bin");
        children("C:\\Program Files\\MySQL", "bin");
        children("C:\\Program Files\\MariaDB", "bin");
    }
    #[cfg(target_os = "linux")]
    {
        children("/usr/lib/postgresql", "bin");
    }
    #[cfg(target_os = "macos")]
    dirs.extend([
        "/opt/homebrew/bin".into(),
        "/usr/local/bin".into(),
        "/usr/local/mysql/bin".into(),
    ]);
    #[cfg(target_os = "linux")]
    dirs.extend(["/usr/bin".into(), "/usr/local/bin".into()]);
    dirs
}

/// Every copy of `name` found, each once.
pub fn candidates(name: &str) -> Vec<PathBuf> {
    let file = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    let mut found: Vec<PathBuf> = Vec::new();
    for path in search_dirs().into_iter().map(|dir| dir.join(&file)) {
        if path.is_file() {
            let real = std::fs::canonicalize(&path).unwrap_or(path);
            if !found.contains(&real) {
                found.push(real);
            }
        }
    }
    found
}

/// The first copy of `name` in the usual places.
pub fn find(name: &str) -> Option<PathBuf> {
    candidates(name).into_iter().next()
}

/// (major, minor) from a tool's or server's version text: "pg_dump
/// (PostgreSQL) 18.0", "mysqldump  Ver 9.3.0 for macos", "PostgreSQL 16.4",
/// "MySQL 8.4.2". The first number with a dot after it.
pub fn parse_version(text: &str) -> Option<(u32, u32)> {
    text.split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter(|word| word.contains('.'))
        .find_map(|word| {
            let mut parts = word.split('.');
            Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
        })
}

pub async fn tool_version(path: &Path) -> Option<(u32, u32)> {
    let out = tokio::process::Command::new(path)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .output()
        .await
        .ok()?;
    parse_version(&String::from_utf8_lossy(&out.stdout))
}

/// Picks the copy of a tool that suits the server: for PostgreSQL the
/// newest (a newer pg_dump reads older servers, never the other way round);
/// for MySQL the same major version, else the nearest newer one, since
/// mysqldump 9 asks MySQL 8 for tables it doesn't have.
pub async fn choose(engine: Engine, name: &str, server: &str) -> Result<PathBuf, AppError> {
    let mut versions = Vec::new();
    for path in candidates(name) {
        if let Some(version) = tool_version(&path).await {
            versions.push((version, path));
        }
    }
    let Some(server_version) = parse_version(server) else {
        return versions
            .into_iter()
            .max_by_key(|(v, _)| *v)
            .map(|(_, p)| p)
            .ok_or_else(|| missing(name));
    };
    if versions.is_empty() {
        return Err(missing(name));
    }
    let picked = match engine {
        Engine::Postgres => {
            let (version, path) = versions.into_iter().max_by_key(|(v, _)| *v).unwrap();
            if version.0 < server_version.0 {
                return Err(AppError::new(
                    "tool_too_old",
                    format!(
                        "The newest {name} here is version {}, older than the server's {}. Install PostgreSQL {}'s client tools.",
                        version.0, server_version.0, server_version.0
                    ),
                ));
            }
            path
        }
        _ => {
            let same = versions
                .iter()
                .filter(|(v, _)| v.0 == server_version.0)
                .max_by_key(|(v, _)| *v);
            let newer = versions
                .iter()
                .filter(|(v, _)| v.0 > server_version.0)
                .min_by_key(|(v, _)| *v);
            let newest = versions.iter().max_by_key(|(v, _)| *v);
            same.or(newer).or(newest).map(|(_, p)| p.clone()).unwrap()
        }
    };
    Ok(picked)
}

pub fn missing(name: &str) -> AppError {
    AppError::new(
        "tool_missing",
        format!(
            "{name} wasn't found on this computer. Install your database's client tools, then try again."
        ),
    )
}

/// What backs up and restores on an engine: (dump, restore SQL, restore archive).
pub fn names(engine: Engine) -> (&'static str, &'static str, Option<&'static str>) {
    match engine {
        Engine::Postgres => ("pg_dump", "psql", Some("pg_restore")),
        Engine::MySql => ("mysqldump", "mysql", None),
        Engine::SqlServer => ("", "", None),
    }
}

/// Where a tool connects: the tunnel's local end when there is one.
pub fn endpoint(params: &ConnectParams) -> (String, u16) {
    match params.connect_to {
        Some(addr) => (addr.ip().to_string(), addr.port()),
        None => (params.host.clone(), params.port),
    }
}

/// A command to run: the arguments and the environment it adds.
#[derive(Debug, Default)]
pub struct Invocation {
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

fn pg_env(params: &ConnectParams) -> Vec<(String, String)> {
    let mut env = vec![
        (
            "PGSSLMODE".to_owned(),
            match params.tls {
                TlsMode::Off => "disable",
                TlsMode::EncryptOnly => "require",
                TlsMode::Verify => "verify-full",
            }
            .to_owned(),
        ),
        ("PGCONNECT_TIMEOUT".to_owned(), "15".to_owned()),
    ];
    if let Some(password) = &params.password {
        env.push(("PGPASSWORD".to_owned(), password.clone()));
    }
    let path = |p: &Path| p.to_string_lossy().into_owned();
    if let Some(ca) = &params.ca_path {
        env.push(("PGSSLROOTCERT".to_owned(), path(ca)));
    }
    if let Some(cert) = &params.client_cert {
        env.push(("PGSSLCERT".to_owned(), path(cert)));
    }
    if let Some(key) = &params.client_key {
        env.push(("PGSSLKEY".to_owned(), path(key)));
    }
    env
}

fn pg_target(params: &ConnectParams, database: &str) -> Vec<String> {
    let (host, port) = endpoint(params);
    vec![
        "--host".into(),
        host,
        "--port".into(),
        port.to_string(),
        "--username".into(),
        params.username.clone(),
        "--dbname".into(),
        database.into(),
        "--no-password".into(),
    ]
}

fn mysql_target(params: &ConnectParams, option_file: &Path) -> Vec<String> {
    let (host, port) = endpoint(params);
    let mut args = vec![
        // Must come first: MySQL's tools only honour it in first place.
        format!("--defaults-extra-file={}", option_file.display()),
        "--host".into(),
        host,
        "--port".into(),
        port.to_string(),
        "--user".into(),
        params.username.clone(),
        format!(
            "--ssl-mode={}",
            match params.tls {
                TlsMode::Off => "DISABLED",
                TlsMode::EncryptOnly => "REQUIRED",
                TlsMode::Verify => "VERIFY_IDENTITY",
            }
        ),
    ];
    if let Some(ca) = &params.ca_path {
        args.push(format!("--ssl-ca={}", ca.display()));
    }
    args
}

/// The backup command for PostgreSQL (custom format) or MySQL (SQL text).
/// `column_statistics` is whether this mysqldump knows that option, which
/// must be turned off to dump MySQL 5.7 with a newer client.
pub fn dump(
    engine: Engine,
    params: &ConnectParams,
    database: &str,
    out: &Path,
    option_file: &Path,
    column_statistics: bool,
    routines: bool,
) -> Invocation {
    match engine {
        Engine::Postgres => {
            let mut args = pg_target(params, database);
            args.extend([
                "--format=custom".into(),
                "--verbose".into(),
                "--file".into(),
                out.to_string_lossy().into_owned(),
            ]);
            Invocation {
                args,
                env: pg_env(params),
            }
        }
        _ => {
            let mut args = mysql_target(params, option_file);
            args.extend([
                "--single-transaction".into(),
                // Tablespaces need the PROCESS privilege, which ordinary
                // logins lack, and a database restores without them.
                "--no-tablespaces".into(),
                "--triggers".into(),
                "--verbose".into(),
                format!("--result-file={}", out.display()),
            ]);
            if column_statistics {
                args.push("--column-statistics=0".into());
            }
            if routines {
                args.push("--routines".into());
            }
            args.push(database.into());
            Invocation { args, env: vec![] }
        }
    }
}

/// The restore command. PostgreSQL restores a custom-format archive with
/// pg_restore (replacing what is there) and a .sql file with psql; MySQL
/// reads the file on standard input.
pub fn restore(
    engine: Engine,
    params: &ConnectParams,
    database: &str,
    input: &Path,
    option_file: &Path,
) -> (Invocation, bool) {
    let sql_text = input
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("sql"));
    match engine {
        Engine::Postgres if sql_text => {
            let mut args = pg_target(params, database);
            args.extend([
                "--set".into(),
                "ON_ERROR_STOP=1".into(),
                "--file".into(),
                input.to_string_lossy().into_owned(),
            ]);
            (
                Invocation {
                    args,
                    env: pg_env(params),
                },
                false,
            )
        }
        Engine::Postgres => {
            let mut args = pg_target(params, database);
            args.extend([
                "--clean".into(),
                "--if-exists".into(),
                "--no-owner".into(),
                "--verbose".into(),
                input.to_string_lossy().into_owned(),
            ]);
            (
                Invocation {
                    args,
                    env: pg_env(params),
                },
                false,
            )
        }
        _ => {
            let mut args = mysql_target(params, option_file);
            args.push(database.into());
            (Invocation { args, env: vec![] }, true)
        }
    }
}

/// The `[client]` file MySQL's tools read the password from.
pub fn option_file_text(params: &ConnectParams) -> String {
    let password = params
        .password
        .as_deref()
        .unwrap_or_default()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!("[client]\npassword=\"{password}\"\n")
}

/// The option file, readable only by this user, removed when dropped.
pub struct OptionFile(pub PathBuf);

impl OptionFile {
    pub fn create(params: &ConnectParams) -> Result<Self, AppError> {
        let path = std::env::temp_dir().join(format!(
            "thabi-{}-{}.cnf",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        use std::io::Write;
        let mut file = options
            .open(&path)
            .map_err(|_| AppError::new("io", "A temporary file couldn't be made."))?;
        file.write_all(option_file_text(params).as_bytes())
            .map_err(|_| AppError::new("io", "A temporary file couldn't be written."))?;
        Ok(Self(path))
    }
}

impl Drop for OptionFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(engine: Engine) -> ConnectParams {
        ConnectParams {
            engine,
            host: "db.internal".into(),
            port: 5432,
            database: None,
            username: "app".into(),
            password: Some("s3cr\"et\\pw".into()),
            tls: TlsMode::Verify,
            ca_path: None,
            client_cert: None,
            client_key: None,
            ssh: None,
            connect_to: Some("127.0.0.1:61000".parse().unwrap()),
            read_only: false,
            startup_commands: vec![],
        }
    }

    #[test]
    fn reads_tool_and_server_versions() {
        assert_eq!(parse_version("pg_dump (PostgreSQL) 18.0"), Some((18, 0)));
        assert_eq!(
            parse_version("mysqldump  Ver 9.3.0 for macos15.2 on arm64"),
            Some((9, 3))
        );
        assert_eq!(parse_version("PostgreSQL 16.4 on aarch64"), Some((16, 4)));
        assert_eq!(parse_version("MySQL 5.7.33-log"), Some((5, 7)));
        assert_eq!(parse_version("no version"), None);
    }

    #[test]
    fn passwords_never_go_on_the_command_line() {
        for engine in [Engine::Postgres, Engine::MySql] {
            let p = params(engine);
            let dumped = dump(
                engine,
                &p,
                "shop",
                Path::new("/tmp/x"),
                Path::new("/tmp/o.cnf"),
                true,
                true,
            );
            let (restored, _) = restore(
                engine,
                &p,
                "shop",
                Path::new("/tmp/x.dump"),
                Path::new("/tmp/o.cnf"),
            );
            for args in [&dumped.args, &restored.args] {
                assert!(
                    !args.iter().any(|a| a.contains("s3cr")),
                    "{engine:?}: {args:?}"
                );
                // Through the tunnel's local end, not the remote host.
                assert!(args.contains(&"127.0.0.1".to_owned()), "{engine:?}");
            }
        }
        let pg = dump(
            Engine::Postgres,
            &params(Engine::Postgres),
            "shop",
            Path::new("/tmp/x"),
            Path::new("/o"),
            false,
            true,
        );
        assert!(
            pg.env
                .contains(&("PGPASSWORD".into(), "s3cr\"et\\pw".into()))
        );
        assert!(pg.env.contains(&("PGSSLMODE".into(), "verify-full".into())));
    }

    #[test]
    fn the_mysql_option_file_quotes_the_password() {
        assert_eq!(
            option_file_text(&params(Engine::MySql)),
            "[client]\npassword=\"s3cr\\\"et\\\\pw\"\n"
        );
        let file = OptionFile::create(&params(Engine::MySql)).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&file.0).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600, "only this user can read it");
        }
        let path = file.0.clone();
        drop(file);
        assert!(!path.exists(), "removed when done");
    }

    #[test]
    fn mysql_restores_from_standard_input_and_postgres_by_format() {
        let p = params(Engine::MySql);
        let (_, stdin) = restore(
            Engine::MySql,
            &p,
            "shop",
            Path::new("/x.sql"),
            Path::new("/o"),
        );
        assert!(stdin);
        let (sql, _) = restore(
            Engine::Postgres,
            &params(Engine::Postgres),
            "shop",
            Path::new("/x.sql"),
            Path::new("/o"),
        );
        assert!(sql.args.contains(&"ON_ERROR_STOP=1".to_owned()));
        let (archive, _) = restore(
            Engine::Postgres,
            &params(Engine::Postgres),
            "shop",
            Path::new("/x.dump"),
            Path::new("/o"),
        );
        assert!(archive.args.contains(&"--clean".to_owned()));
    }
}

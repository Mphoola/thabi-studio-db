//! Backup and restore with each engine's own tools (Phase 15): pg_dump and
//! pg_restore or psql, mysqldump and mysql, run as child processes with the
//! connection's credentials; and on SQL Server, the server's own BACKUP
//! DATABASE and RESTORE, which write and read a path on the server.
//!
//! A connection over SSH gets a tunnel of its own for the tool to go
//! through, open for as long as the tool runs.

pub mod tools;

use crate::database::{ConnectParams, Engine, Session, quote};
use crate::errors::AppError;
use serde::Serialize;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::sync::CancellationToken;

const LOG_LINES: usize = 400;

/// A backup or restore while it runs: its log, and a way to stop it.
#[derive(Default)]
pub struct Job {
    log: Mutex<VecDeque<String>>,
    pub cancel: CancellationToken,
}

impl Job {
    fn push(&self, line: String) {
        if let Ok(mut log) = self.log.lock() {
            if log.len() == LOG_LINES {
                log.pop_front();
            }
            log.push_back(line);
        }
    }

    pub fn log(&self) -> Vec<String> {
        self.log
            .lock()
            .map(|log| log.iter().cloned().collect())
            .unwrap_or_default()
    }
}

/// Which tools this computer has for an engine.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub dump: Option<String>,
    pub restore: Option<String>,
    pub archive_restore: Option<String>,
    /// SQL Server: backups are made by the server, to a path on the server.
    pub server_side: bool,
}

pub fn status(engine: Engine) -> ToolStatus {
    let (dump, restore, archive) = tools::names(engine);
    let found = |name: &str| {
        (!name.is_empty())
            .then(|| tools::find(name))
            .flatten()
            .map(|p| p.to_string_lossy().into_owned())
    };
    ToolStatus {
        dump: found(dump),
        restore: found(restore),
        archive_restore: archive.and_then(found),
        server_side: engine == Engine::SqlServer,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResult {
    pub path: String,
    pub bytes: Option<u64>,
    pub log: Vec<String>,
    /// What the backup or restore couldn't do, said plainly.
    pub warnings: Vec<String>,
}

/// A session of the tool's own: it checks the login first, says which
/// server version the tool must suit, and, over SSH, keeps a tunnel open for
/// the tool to go through while it runs.
async fn reachable(params: &ConnectParams) -> Result<(ConnectParams, Session, String), AppError> {
    let session = Session::connect(params).await?;
    let version = session.server_info().version.clone();
    let mut through = params.clone();
    through.connect_to = session.tunnel_addr();
    Ok((through, session, version))
}

/// Runs a tool to the end, its standard error going to the job's log.
async fn run_tool(
    tool: &Path,
    invocation: &tools::Invocation,
    stdin: Option<&Path>,
    secret: Option<&str>,
    job: &Job,
) -> Result<(), AppError> {
    let mut command = tokio::process::Command::new(tool);
    command
        .args(&invocation.args)
        .envs(invocation.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    match stdin {
        Some(path) => {
            let file = std::fs::File::open(path)
                .map_err(|_| AppError::new("io", "That file couldn't be opened."))?;
            command.stdin(Stdio::from(file));
        }
        None => {
            command.stdin(Stdio::null());
        }
    }
    let mut child = command.spawn().map_err(|_| {
        AppError::new(
            "tool_failed",
            format!("{} couldn't be started.", tool.display()),
        )
    })?;
    let stderr = child.stderr.take();
    let reading = async {
        if let Some(stderr) = stderr {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let line = match secret {
                    Some(secret) if !secret.is_empty() => line.replace(secret, "••••"),
                    _ => line,
                };
                job.push(line);
            }
        }
    };
    let status = tokio::select! {
        (status, ()) = async { tokio::join!(child.wait(), reading) } => status,
        () = job.cancel.cancelled() => {
            let _ = child.kill().await;
            return Err(AppError::new("cancelled", "Stopped."));
        }
    };
    let status =
        status.map_err(|_| AppError::new("tool_failed", "The tool stopped unexpectedly."))?;
    if !status.success() && !only_benign_errors(&job.log()) {
        let tail = job.log().into_iter().rev().take(6).collect::<Vec<_>>();
        let detail = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
        return Err(AppError::new(
            "tool_failed",
            format!(
                "{} stopped with an error.",
                tool.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ),
        )
        .with_detail(Some(detail)));
    }
    Ok(())
}

/// pg_restore 17 and later set `transaction_timeout` before anything else,
/// which servers before 17 don't know: one error, and nothing else wrong
/// with the restore. A run whose only errors are that one has succeeded.
fn only_benign_errors(log: &[String]) -> bool {
    let errors: Vec<&String> = log
        .iter()
        .filter(|line| line.contains("error:") && !line.contains("errors ignored"))
        .collect();
    !errors.is_empty()
        && errors
            .iter()
            .all(|line| line.contains("transaction_timeout") || line.contains("while INITIALIZING"))
        && log
            .iter()
            .any(|line| line.contains("errors ignored on restore: 1"))
}

/// mysqldump 8 and later ask MySQL 5.7 for statistics it doesn't have
/// unless told not to; MariaDB's mysqldump doesn't know the option at all.
async fn knows_column_statistics(tool: &Path) -> bool {
    tokio::process::Command::new(tool)
        .arg("--help")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await
        .map(|out| String::from_utf8_lossy(&out.stdout).contains("column-statistics"))
        .unwrap_or(false)
}

/// Backs a database up to `out`.
pub async fn backup(
    params: &ConnectParams,
    database: &str,
    out: PathBuf,
    job: &Job,
) -> Result<JobResult, AppError> {
    let engine = params.engine;
    if engine == Engine::SqlServer {
        let mut session = Session::connect(params).await?;
        let sql = format!(
            "BACKUP DATABASE {} TO DISK = {} WITH COPY_ONLY, INIT, STATS = 10",
            quote::ident(engine, database),
            quote::literal(engine, &out.to_string_lossy())
        );
        job.push(format!("Backing up {database} on the server…"));
        let result = session.execute(&sql, 0).await;
        let _ = session.close().await;
        result?;
        job.push("Done.".into());
        return Ok(JobResult {
            path: out.to_string_lossy().into_owned(),
            bytes: None,
            log: job.log(),
            warnings: Vec::new(),
        });
    }
    let (name, _, _) = tools::names(engine);
    let (through, tunnel, server) = reachable(params).await?;
    let tool = tools::choose(engine, name, &server).await?;
    job.push(format!("Using {}", tool.display()));
    let option_file = tools::OptionFile::create(&through)?;
    let column_statistics = engine == Engine::MySql && knows_column_statistics(&tool).await;
    // mysqldump 9 reads stored routines from a table MySQL 8 and older don't
    // have, and fails. With only a newer client here, routines are left out
    // and the result says so, rather than there being no backup at all.
    let mut warnings = Vec::new();
    let tool_major = tools::tool_version(&tool).await.map(|v| v.0);
    let server_major = tools::parse_version(&server).map(|v| v.0);
    let routines = !(engine == Engine::MySql
        && matches!((tool_major, server_major), (Some(t), Some(s)) if t >= 9 && s < 9));
    if !routines {
        warnings.push(format!(
            "Stored procedures and functions aren't in this backup: the only mysqldump here is version {}, which can't read them from MySQL {}. Install a MySQL {} client to include them.",
            tool_major.unwrap_or_default(),
            server_major.unwrap_or_default(),
            server_major.unwrap_or_default()
        ));
    }
    let invocation = tools::dump(
        engine,
        &through,
        database,
        &out,
        &option_file.0,
        column_statistics,
        routines,
    );
    let result = run_tool(&tool, &invocation, None, params.password.as_deref(), job).await;
    drop(option_file);
    let _ = tunnel.close().await;
    if result.is_err() {
        // A half-written backup looks like a backup; it isn't one.
        let _ = std::fs::remove_file(&out);
    }
    result?;
    Ok(JobResult {
        bytes: std::fs::metadata(&out).map(|m| m.len()).ok(),
        path: out.to_string_lossy().into_owned(),
        log: job.log(),
        warnings,
    })
}

/// Restores `input` into a database. On SQL Server the path is on the
/// server, and other connections to the database are closed first, because
/// RESTORE needs it to itself.
pub async fn restore(
    params: &ConnectParams,
    database: &str,
    input: PathBuf,
    job: &Job,
) -> Result<JobResult, AppError> {
    let engine = params.engine;
    if engine == Engine::SqlServer {
        let mut master = params.clone();
        master.database = Some("master".into());
        let mut session = Session::connect(&master).await?;
        let db = quote::ident(engine, database);
        let steps = [
            format!(
                "IF DB_ID({}) IS NOT NULL ALTER DATABASE {db} SET SINGLE_USER WITH ROLLBACK IMMEDIATE",
                quote::literal(engine, database)
            ),
            format!(
                "RESTORE DATABASE {db} FROM DISK = {} WITH REPLACE, STATS = 10",
                quote::literal(engine, &input.to_string_lossy())
            ),
            format!("ALTER DATABASE {db} SET MULTI_USER"),
        ];
        let mut outcome = Ok(());
        for step in &steps {
            job.push(step.clone());
            if let Err(error) = session.execute(step, 0).await {
                outcome = Err(error);
                break;
            }
        }
        if outcome.is_err() {
            // Never leave the database stuck in single-user mode.
            let _ = session.execute(&steps[2], 0).await;
        }
        let _ = session.close().await;
        outcome?;
        job.push("Done.".into());
        return Ok(JobResult {
            path: input.to_string_lossy().into_owned(),
            bytes: None,
            log: job.log(),
            warnings: Vec::new(),
        });
    }
    let (_, sql_tool, archive_tool) = tools::names(engine);
    let (through, tunnel, server) = reachable(params).await?;
    let option_file = tools::OptionFile::create(&through)?;
    let (invocation, stdin) = tools::restore(engine, &through, database, &input, &option_file.0);
    let name = if engine == Engine::Postgres
        && !input
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("sql"))
    {
        archive_tool.unwrap_or(sql_tool)
    } else {
        sql_tool
    };
    let tool = tools::choose(engine, name, &server).await?;
    job.push(format!("Using {}", tool.display()));
    let result = run_tool(
        &tool,
        &invocation,
        stdin.then_some(input.as_path()),
        params.password.as_deref(),
        job,
    )
    .await;
    drop(option_file);
    let _ = tunnel.close().await;
    result?;
    Ok(JobResult {
        path: input.to_string_lossy().into_owned(),
        bytes: None,
        log: job.log(),
        warnings: Vec::new(),
    })
}

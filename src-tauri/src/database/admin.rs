//! Looking after a server: who is connected and what they are running,
//! stopping a runaway query, the server's vital signs, and where the space
//! goes (Phase 15).
//!
//! Every query reads the server's own catalog views. Values are bound; the
//! one exception is a session id for KILL, which MySQL and SQL Server take
//! only as a literal, so it is checked to be a number first.

use super::{Cell, Engine, Session, quote, values::Param};
use crate::errors::AppError;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub id: String,
    pub user: String,
    pub database: String,
    pub client: String,
    /// active, idle, sleeping, …: each engine's own word.
    pub state: String,
    /// What it is waiting on, when the engine says.
    pub waiting: String,
    /// Seconds in the current state or statement.
    pub seconds: Option<i64>,
    pub query: String,
    /// This app's own session, which can't sensibly be killed from here.
    pub own: bool,
}

fn text(row: &[Cell], i: usize) -> String {
    match row.get(i) {
        Some(Cell::Value(v)) => v.clone(),
        _ => String::new(),
    }
}

fn number(row: &[Cell], i: usize) -> Option<i64> {
    match row.get(i) {
        Some(Cell::Value(v)) => v.trim().parse::<f64>().ok().map(|n| n as i64),
        _ => None,
    }
}

fn truthy(row: &[Cell], i: usize) -> bool {
    matches!(text(row, i).as_str(), "1" | "true" | "t")
}

/// The sessions on the server, longest-running first.
pub async fn processes(session: &mut Session) -> Result<Vec<ProcessInfo>, AppError> {
    let sql = match session.server_info().engine {
        Engine::Postgres => {
            "SELECT pid::text, usename::text, datname::text, coalesce(client_addr::text, 'local'),
                    coalesce(state, ''), coalesce(wait_event_type || ': ' || wait_event, ''),
                    EXTRACT(EPOCH FROM (now() - coalesce(query_start, backend_start)))::bigint,
                    coalesce(query, ''), (pid = pg_backend_pid())::text
             FROM pg_stat_activity WHERE backend_type = 'client backend'
             ORDER BY 7 DESC NULLS LAST"
        }
        Engine::MySql => {
            "SELECT ID, CONVERT(USER USING utf8mb4), CONVERT(COALESCE(DB, '') USING utf8mb4),
                    CONVERT(HOST USING utf8mb4), CONVERT(COMMAND USING utf8mb4),
                    CONVERT(COALESCE(STATE, '') USING utf8mb4), TIME,
                    CONVERT(COALESCE(INFO, '') USING utf8mb4), IF(ID = CONNECTION_ID(), 1, 0)
             FROM information_schema.PROCESSLIST ORDER BY TIME DESC"
        }
        Engine::SqlServer => {
            "SELECT CAST(s.session_id AS nvarchar(10)), s.login_name,
                    COALESCE(DB_NAME(COALESCE(r.database_id, s.database_id)), ''),
                    COALESCE(s.host_name, ''), COALESCE(r.status, s.status), COALESCE(r.wait_type, ''),
                    DATEDIFF(SECOND, COALESCE(r.start_time, s.last_request_start_time), SYSDATETIME()),
                    COALESCE(t.text, ''), IIF(s.session_id = @@SPID, 1, 0)
             FROM sys.dm_exec_sessions s
             LEFT JOIN sys.dm_exec_requests r ON r.session_id = s.session_id
             OUTER APPLY sys.dm_exec_sql_text(r.sql_handle) t
             WHERE s.is_user_process = 1
             ORDER BY 7 DESC"
        }
    };
    let result = session.execute(sql, 10_000).await?;
    Ok(result
        .rows
        .iter()
        .map(|row| ProcessInfo {
            id: text(row, 0),
            user: text(row, 1),
            database: text(row, 2),
            client: text(row, 3),
            state: text(row, 4),
            waiting: text(row, 5),
            seconds: number(row, 6),
            query: text(row, 7),
            own: truthy(row, 8),
        })
        .collect())
}

/// The statement that stops a session's current query (`terminate` false)
/// or ends the session. SQL Server has only the second.
pub fn kill_sql(
    engine: Engine,
    id: &str,
    terminate: bool,
) -> Result<(String, Vec<Param>), AppError> {
    let id = id.trim();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::new("validation", "That isn't a session id."));
    }
    Ok(match engine {
        Engine::Postgres => (
            format!(
                "SELECT {}($1::text::int)",
                if terminate {
                    "pg_terminate_backend"
                } else {
                    "pg_cancel_backend"
                }
            ),
            vec![Param::Text(id.to_owned())],
        ),
        // Checked to be digits above; KILL takes no bound value.
        Engine::MySql => (
            format!("KILL {}{id}", if terminate { "" } else { "QUERY " }),
            vec![],
        ),
        Engine::SqlServer => (format!("KILL {id}"), vec![]),
    })
}

pub async fn kill(session: &mut Session, id: &str, terminate: bool) -> Result<(), AppError> {
    let engine = session.server_info().engine;
    if engine == Engine::SqlServer && !terminate {
        return Err(AppError::new(
            "unsupported",
            "SQL Server can only end the whole session, not cancel its query.",
        ));
    }
    let (sql, params) = kill_sql(engine, id, terminate)?;
    if params.is_empty() {
        session.execute(&sql, 0).await?;
    } else {
        let result = session.execute_bound(&sql, &params, 1).await?;
        // PostgreSQL answers false when there was no such session to signal.
        if matches!(result.rows.first().and_then(|r| r.first()), Some(Cell::Value(v)) if v == "false" || v == "f")
        {
            return Err(AppError::new(
                "not_found",
                "That session has already ended.",
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerFact {
    pub label: String,
    pub value: String,
}

/// Version, uptime, connections and the settings people look for first.
pub async fn overview(session: &mut Session) -> Result<Vec<ServerFact>, AppError> {
    let (sql, labels): (&str, &[&str]) = match session.server_info().engine {
        Engine::Postgres => (
            "SELECT version(),
                    date_trunc('second', now() - pg_postmaster_start_time())::text,
                    (SELECT count(*) FROM pg_stat_activity WHERE backend_type = 'client backend')::text,
                    current_setting('max_connections'),
                    pg_size_pretty(pg_database_size(current_database())),
                    current_setting('shared_buffers'), current_setting('work_mem'),
                    current_setting('TimeZone')",
            &[
                "Version",
                "Up for",
                "Connections",
                "Max connections",
                "This database",
                "shared_buffers",
                "work_mem",
                "Time zone",
            ],
        ),
        Engine::MySql => (
            "SELECT CONCAT('MySQL ', @@version, ' (', @@version_comment, ')'),
                    SEC_TO_TIME((SELECT VARIABLE_VALUE FROM performance_schema.global_status
                                 WHERE VARIABLE_NAME = 'Uptime')),
                    (SELECT VARIABLE_VALUE FROM performance_schema.global_status
                     WHERE VARIABLE_NAME = 'Threads_connected'),
                    @@max_connections,
                    CONCAT(ROUND(@@innodb_buffer_pool_size / 1048576), ' MB'),
                    @@sql_mode, @@time_zone",
            &[
                "Version",
                "Up for",
                "Connections",
                "Max connections",
                "InnoDB buffer pool",
                "sql_mode",
                "Time zone",
            ],
        ),
        Engine::SqlServer => (
            "SELECT REPLACE(LEFT(@@VERSION, CHARINDEX(CHAR(10), @@VERSION + CHAR(10)) - 1), CHAR(13), ''),
                    CONCAT(DATEDIFF(HOUR, sqlserver_start_time, SYSDATETIME()) / 24, ' days ',
                           DATEDIFF(HOUR, sqlserver_start_time, SYSDATETIME()) % 24, ' hours'),
                    (SELECT COUNT(*) FROM sys.dm_exec_sessions WHERE is_user_process = 1),
                    @@MAX_CONNECTIONS,
                    CONCAT(physical_memory_kb / 1024, ' MB'),
                    cpu_count,
                    CAST(SERVERPROPERTY('Edition') AS nvarchar(128)),
                    CAST(SERVERPROPERTY('Collation') AS nvarchar(128))
             FROM sys.dm_os_sys_info",
            &[
                "Version",
                "Up for",
                "Connections",
                "Max connections",
                "Memory",
                "CPUs",
                "Edition",
                "Collation",
            ],
        ),
    };
    let result = session.execute(sql, 1).await?;
    let row = result.rows.first().cloned().unwrap_or_default();
    Ok(labels
        .iter()
        .enumerate()
        .map(|(i, label)| ServerFact {
            label: (*label).to_owned(),
            value: text(&row, i),
        })
        .collect())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableSize {
    pub schema: String,
    pub name: String,
    /// The catalog's estimate; exact counts would read every row.
    pub rows: Option<i64>,
    pub data_bytes: Option<i64>,
    pub index_bytes: Option<i64>,
    pub total_bytes: Option<i64>,
}

/// Tables in a schema (a database on MySQL) by size, largest first.
pub async fn storage(
    session: &mut Session,
    database: &str,
    schema: &str,
) -> Result<Vec<TableSize>, AppError> {
    let engine = session.server_info().engine;
    let (sql, params): (String, Vec<Param>) = match engine {
        Engine::Postgres => {
            let current = session.server_info().database.clone().unwrap_or_default();
            if current != database {
                return Err(super::error::database_not_open(database, &current));
            }
            (
                "SELECT n.nspname::text, c.relname::text, greatest(c.reltuples, 0)::bigint,
                        pg_table_size(c.oid), pg_indexes_size(c.oid), pg_total_relation_size(c.oid)
                 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = $1 AND c.relkind IN ('r', 'p', 'm')
                 ORDER BY 6 DESC, 2"
                    .into(),
                vec![Param::Text(schema.into())],
            )
        }
        Engine::MySql => (
            "SELECT CONVERT(TABLE_SCHEMA USING utf8mb4), CONVERT(TABLE_NAME USING utf8mb4),
                    TABLE_ROWS, DATA_LENGTH, INDEX_LENGTH, DATA_LENGTH + INDEX_LENGTH
             FROM information_schema.TABLES
             WHERE TABLE_SCHEMA = ? AND TABLE_TYPE = 'BASE TABLE'
             ORDER BY 6 DESC, 2"
                .into(),
            vec![Param::Text(database.into())],
        ),
        Engine::SqlServer => {
            let db = quote::ident(engine, database);
            (
                format!(
                    "SELECT s.name, t.name,
                            SUM(CASE WHEN p.index_id IN (0, 1) THEN p.rows ELSE 0 END),
                            SUM(CASE WHEN p.index_id IN (0, 1) THEN a.used_pages ELSE 0 END) * 8192,
                            SUM(CASE WHEN p.index_id > 1 THEN a.used_pages ELSE 0 END) * 8192,
                            SUM(a.used_pages) * 8192
                     FROM {db}.sys.tables t
                     JOIN {db}.sys.schemas s ON s.schema_id = t.schema_id
                     JOIN {db}.sys.partitions p ON p.object_id = t.object_id
                     JOIN {db}.sys.allocation_units a ON a.container_id = p.partition_id
                     WHERE s.name = @P1
                     GROUP BY s.name, t.name
                     ORDER BY 6 DESC, 2"
                ),
                vec![Param::Text(schema.into())],
            )
        }
    };
    if engine == Engine::MySql {
        // MySQL serves these figures from a cache that can be a day old, or
        // empty for a table it hasn't analysed: ask for current ones. MySQL
        // before 8.0 has no such cache, nor the setting.
        let _ = session
            .execute("SET SESSION information_schema_stats_expiry = 0", 0)
            .await;
    }
    let result = session.execute_bound(&sql, &params, 100_000).await?;
    Ok(result
        .rows
        .iter()
        .map(|row| TableSize {
            schema: text(row, 0),
            name: text(row, 1),
            rows: number(row, 2),
            data_bytes: number(row, 3),
            index_bytes: number(row, 4),
            total_bytes: number(row, 5),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kill_takes_only_a_session_number() {
        assert!(kill_sql(Engine::MySql, "12; DROP TABLE x", true).is_err());
        assert!(kill_sql(Engine::SqlServer, "", true).is_err());
        assert_eq!(
            kill_sql(Engine::MySql, "42", false).unwrap().0,
            "KILL QUERY 42"
        );
        assert_eq!(kill_sql(Engine::MySql, "42", true).unwrap().0, "KILL 42");
        assert_eq!(
            kill_sql(Engine::SqlServer, "57", true).unwrap().0,
            "KILL 57"
        );
        let (sql, params) = kill_sql(Engine::Postgres, "7", false).unwrap();
        assert!(sql.contains("pg_cancel_backend"));
        assert_eq!(params.len(), 1);
    }
}

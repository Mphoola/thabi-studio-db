use crate::{
    connections::{ConnectionStatus, SessionHandle},
    database::{
        self, DbObject, ObjectKind, ObjectStructure, QueryResult, Session, TestResult, values::Cell,
    },
    errors::AppError,
    profiles::{self, ProfileInput},
    queries::QueryTabHandle,
    state::AppState,
};
use serde::Serialize;
use tauri::{Emitter, State};

/// Default rows fetched per statement; the user can raise it in Settings.
const DEFAULT_MAX_ROWS: usize = 10_000;
/// Upper bound on rows kept per statement, whatever the setting says.
const MAX_ROWS_LIMIT: usize = 1_000_000;
/// Rows a table tab reads from the server at a time (Phase 12).
const DEFAULT_PAGE_ROWS: usize = 200;

/// One page of a table, as the grid's source needs it.
///
/// Deliberately not a kept `QueryResult`: a table's pages are read from the
/// server on demand, so there is nothing to keep in Rust between them. A query
/// tab's result is the opposite and still keeps its rows (`result_id`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectPage {
    pub columns: Vec<database::values::ColumnMeta>,
    pub rows: Vec<Vec<Cell>>,
    /// The page was cut short at `limit`, so another page may exist.
    pub truncated: bool,
    /// Catalog estimate of the table's rows; `None` means unknown.
    pub estimated_total: Option<u64>,
    /// Exact count, once the user asks for it.
    pub total_rows: Option<u64>,
    /// Always `None` for a table page: pages come from the server, not a store.
    pub result_id: Option<String>,
    pub elapsed_ms: u64,
}

/// Tests the values currently in the connection form. Nothing is saved.
/// `profile_id` lets an edited profile reuse its saved password.
#[tauri::command]
pub async fn test_connection_input(
    input: ProfileInput,
    profile_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<TestResult, AppError> {
    let validated = profiles::validate(input)?;
    let params = state
        .blocking(move |connection, credentials| {
            profiles::connect_params_for_input(
                connection,
                credentials,
                profile_id.as_deref(),
                &validated,
            )
        })
        .await?;
    database::test_connection(&params).await
}

/// Connects a saved profile. `database` opens a different database than the
/// profile's default for this connection only (PostgreSQL can only browse
/// the database it is connected to); any open connection is replaced.
/// Secrets the user typed for this one connection. They are used for this
/// session only: never saved, never logged, and gone when it closes.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OneOffSecrets {
    pub database_password: Option<String>,
    pub ssh_password: Option<String>,
    pub ssh_key_passphrase: Option<String>,
}

impl OneOffSecrets {
    /// Whether the user has supplied the secret a profile asks for.
    fn has(&self, secret: &str) -> bool {
        match secret {
            "databasePassword" => self.database_password.is_some(),
            "sshPassword" => self.ssh_password.is_some(),
            "sshKeyPassphrase" => self.ssh_key_passphrase.is_some(),
            _ => true,
        }
    }
}

/// Says which secret the app needs before it can connect, without echoing it.
fn secret_required(secret: &str, connection: &str) -> AppError {
    AppError::new(
        "secret_required",
        format!("Enter the {secret} for “{connection}” to connect."),
    )
    .with_data(serde_json::json!({ "secret": secret, "connection": connection }))
}

#[tauri::command]
pub async fn connect_connection_profile(
    id: String,
    database: Option<String>,
    secrets: Option<OneOffSecrets>,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, AppError> {
    let lookup = id.clone();
    let (profile, mut params) = state
        .blocking(move |connection, credentials| {
            profiles::connect_params(connection, credentials, &lookup)
        })
        .await?;
    let secrets = secrets.unwrap_or_default();
    // A profile set to ask every time keeps nothing, so the app asks.
    if let Some(secret) = profiles::required_secret(&profile)
        && !secrets.has(secret)
    {
        return Err(secret_required(secret, &profile.name));
    }
    profiles::apply_one_off_secrets(&mut params, &secrets);
    if let Some(database) = database.filter(|d| !d.trim().is_empty()) {
        state.connections.disconnect(&id).await?;
        params.database = Some(database);
    }
    let status = state.connections.connect(&id, &params).await?;
    // Best effort: the connection is open even if the timestamp can't be saved.
    let _ = state
        .blocking(move |connection, _| profiles::mark_connected(connection, &id))
        .await;
    Ok(status)
}

#[tauri::command]
pub async fn disconnect_connection_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.connections.disconnect(&id).await
}

#[tauri::command]
pub async fn list_connections(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionStatus>, AppError> {
    Ok(state.connections.statuses().await)
}

/// Errors that mean the session itself is gone rather than the request failing.
fn is_connection_failure(error: &AppError) -> bool {
    matches!(
        error.kind,
        "connection_closed" | "connection_refused" | "unreachable" | "connection" | "timeout"
    )
}

/// Runs explorer work on a profile's session. If the session turns out to
/// be dead, it is closed and the caller gets a "connection lost" error so
/// the UI can offer to reconnect instead of showing stale data.
async fn with_session<T>(
    state: &AppState,
    id: &str,
    work: impl AsyncFnOnce(&mut Session) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let handle: SessionHandle = state.connections.session(id).await?;
    let result = {
        let mut guard = handle.lock().await;
        let session = guard.as_mut().ok_or_else(AppError::not_connected)?;
        work(session).await
    };
    match result {
        Err(error) if is_connection_failure(&error) => {
            let _ = state.connections.disconnect(id).await;
            Err(database::error::connection_lost())
        }
        other => other,
    }
}

#[tauri::command]
pub async fn list_databases(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    with_session(&state, &id, async |s| s.list_databases().await).await
}

/// Schemas in a database, or null when the engine has none (MySQL).
#[tauri::command]
pub async fn list_schemas(
    id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Option<Vec<String>>, AppError> {
    with_session(&state, &id, async |s| s.list_schemas(&database).await).await
}

#[tauri::command]
pub async fn list_objects(
    id: String,
    database: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<DbObject>, AppError> {
    let schema = schema.unwrap_or_default();
    with_session(&state, &id, async |s| {
        s.list_objects(&database, &schema).await
    })
    .await
}

/// Everything the ER diagram draws for one schema.
///
/// Read-only by construction: it returns data, and there is no command that
/// draws a diagram back.
#[tauri::command]
pub async fn schema_diagram(
    id: String,
    database: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<database::diagram::Diagram, AppError> {
    let schema = schema.unwrap_or_default();
    with_session(&state, &id, async |s| s.diagram(&database, &schema).await).await
}

#[tauri::command]
pub async fn describe_object(
    id: String,
    database: String,
    schema: Option<String>,
    name: String,
    kind: String,
    state: State<'_, AppState>,
) -> Result<ObjectStructure, AppError> {
    let schema = schema.unwrap_or_default();
    let kind = ObjectKind::from_code(&kind);
    with_session(&state, &id, async |s| {
        s.describe(&database, &schema, &name, kind).await
    })
    .await
}

/// Reads a table or view into an object tab's own session, so its rows
/// scroll, sort and export like a query result. Bounded by `max_rows`.
///
/// Phase 12: this returns one page plus the catalog's row estimate. Further
/// pages and server-side sorting come from `fetch_table_page`.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri maps each argument from the frontend.
pub async fn open_object_data(
    id: String,
    tab_id: String,
    database: String,
    schema: Option<String>,
    name: String,
    max_rows: Option<usize>,
    filters: Option<Vec<database::filter::RowFilter>>,
    primary_key: Option<Vec<String>>,
    order: Option<database::paging::PageOrder>,
    state: State<'_, AppState>,
) -> Result<ObjectPage, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let filter =
        database::filter::where_clause(params.engine, filters.as_deref().unwrap_or_default())?;
    let limit = max_rows
        .unwrap_or(DEFAULT_PAGE_ROWS)
        .clamp(1, MAX_ROWS_LIMIT);
    let schema = schema.unwrap_or_default();
    let primary_key = primary_key.unwrap_or_default();
    let estimated = table_estimate(&tab, &params, &database, &schema, &name).await;

    let sql = database::paging::page_sql(
        params.engine,
        &database,
        &schema,
        &name,
        &filter,
        order.as_ref(),
        &primary_key,
        0,
        limit + 1,
    );
    let result = tab.run(&params, Some(database), &sql, limit, None).await;
    let result = match result {
        Ok(result) => result,
        Err(error) => {
            if is_connection_failure(&error) {
                state.connections.close_query_tab(&id, &tab_id).await;
                return Err(database::error::connection_lost());
            }
            return Err(error);
        }
    };
    Ok(ObjectPage {
        columns: result.columns,
        rows: result.rows,
        truncated: result.truncated,
        estimated_total: estimated,
        total_rows: None,
        result_id: None,
        elapsed_ms: result.elapsed_ms,
    })
}

/// One page of a table as the grid asks for it, read from the server.
///
/// Sorting re-queries rather than reordering rows already fetched, so the
/// whole table is sortable and not just the page that happened to be loaded.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri maps each argument from the frontend.
pub async fn fetch_table_page(
    id: String,
    tab_id: String,
    database: String,
    schema: Option<String>,
    name: String,
    filters: Option<Vec<database::filter::RowFilter>>,
    primary_key: Option<Vec<String>>,
    order: Option<database::paging::PageOrder>,
    offset: usize,
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<Vec<Cell>>, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let filter =
        database::filter::where_clause(params.engine, filters.as_deref().unwrap_or_default())?;
    // A table page is the chunk the user moves between with the page buttons,
    // sized by the connection's row limit, so it is not the 200 rows a query
    // result sends first. It is still bounded, so one request cannot ask for a
    // whole table.
    let limit = limit.clamp(1, database::paging::MAX_TABLE_PAGE);
    let sql = database::paging::page_sql(
        params.engine,
        &database,
        &schema.unwrap_or_default(),
        &name,
        &filter,
        order.as_ref(),
        primary_key.as_deref().unwrap_or_default(),
        offset,
        limit,
    );
    let result = tab
        .run(&params, Some(database), &sql, limit, None)
        .await
        .map_err(|error| {
            if is_connection_failure(&error) {
                AppError::new(
                    "connection_lost",
                    "The connection to this server was lost. Reconnect and try again.",
                )
            } else {
                error
            }
        })?;
    Ok(result.rows)
}

/// The exact number of rows, when the user asks for it instead of the
/// catalog's estimate.
#[tauri::command]
pub async fn count_table_rows(
    id: String,
    tab_id: String,
    database: String,
    schema: Option<String>,
    name: String,
    filters: Option<Vec<database::filter::RowFilter>>,
    state: State<'_, AppState>,
) -> Result<u64, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let filter =
        database::filter::where_clause(params.engine, filters.as_deref().unwrap_or_default())?;
    let sql = database::paging::count_sql(
        params.engine,
        &database,
        &schema.unwrap_or_default(),
        &name,
        &filter,
    );
    let result = tab.run(&params, Some(database), &sql, 1, None).await?;
    Ok(result
        .rows
        .first()
        .and_then(|row| row.first())
        .and_then(|cell| match cell {
            Cell::Value(value) => value.parse::<u64>().ok(),
            _ => None,
        })
        .unwrap_or_default())
}

/// The catalog's estimate of a table's rows, or `None` when it has none.
///
/// Best effort: a view, a database without statistics, or a permission the
/// catalog denies must not stop the rows themselves from loading, so a failure
/// here is swallowed and reported as "unknown".
async fn table_estimate(
    tab: &QueryTabHandle,
    params: &database::ConnectParams,
    database: &str,
    schema: &str,
    name: &str,
) -> Option<u64> {
    let sql = database::paging::estimate_sql(params.engine, database, schema, name);
    let result = tab
        .run(params, Some(database.to_owned()), &sql, 1, None)
        .await
        .ok()?;
    match result.rows.first()?.first()? {
        Cell::Value(value) => value.parse::<u64>().ok(),
        _ => None,
    }
}

/// The statements staged row edits would run, for the user to review.
#[tauri::command]
pub async fn preview_row_changes(
    id: String,
    tab_id: String,
    target: database::edit::TableTarget,
    changes: Vec<database::edit::RowChange>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let (_, params) = state.connections.query_tab(&id, &tab_id).await?;
    database::edit::statements(params.engine, &target, &changes)
}

/// Values a foreign key column may take, from the referenced table,
/// filtered by `search`. Runs on the table tab's session.
#[tauri::command]
pub async fn lookup_reference(
    id: String,
    tab_id: String,
    database: String,
    reference: database::metadata::ColumnRef,
    search: String,
    state: State<'_, AppState>,
) -> Result<Vec<database::edit::LookupRow>, AppError> {
    use database::{Cell, edit};
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let engine = params.engine;
    tab.with_session(&params, Some(database.clone()), async |s| {
        let (db, schema) = match engine {
            database::Engine::MySql => (reference.schema.as_str(), ""),
            _ => (database.as_str(), reference.schema.as_str()),
        };
        let structure = s
            .describe(db, schema, &reference.table, ObjectKind::Table)
            .await?;
        let label = edit::label_column(
            &reference.column,
            structure.columns.iter().map(|c| c.name.as_str()),
        );
        let sql = edit::lookup_sql(engine, &database, &reference, label, &search);
        let result = s.execute(&sql, edit::LOOKUP_LIMIT).await?;
        let text = |cell: &Cell| match cell {
            Cell::Value(v) => Some(v.clone()),
            _ => None,
        };
        Ok(result
            .rows
            .iter()
            .filter_map(|row| {
                Some(edit::LookupRow {
                    value: text(row.first()?)?,
                    label: row.get(1).and_then(text),
                })
            })
            .collect())
    })
    .await
}

/// Tables and columns of a database for the editor's autocomplete, read on
/// the query tab's session so it is the database the tab runs in.
#[tauri::command]
pub async fn completion_schema(
    id: String,
    tab_id: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<database::metadata::CompletionTable>, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let database = database
        .filter(|d| !d.is_empty())
        .or_else(|| params.database.clone())
        .unwrap_or_default();
    tab.with_session(&params, Some(database.clone()), async |s| {
        s.completion_schema(&database).await
    })
    .await
}

/// The plan for one statement. A plain EXPLAIN uses the tab's session (it
/// sees the tab's temp tables). ANALYZE (PostgreSQL) really runs the
/// statement, so it uses a short separate session inside a transaction that
/// is rolled back, leaving the tab's own transaction alone.
#[tauri::command]
pub async fn explain_statement(
    id: String,
    tab_id: String,
    database: Option<String>,
    sql: String,
    analyze: bool,
    state: State<'_, AppState>,
) -> Result<database::plan::Plan, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let engine = params.engine;
    let analyze = analyze && engine == database::Engine::Postgres;
    if analyze {
        let mut own = params.clone();
        own.database = database.filter(|d| !d.is_empty()).or(own.database);
        let mut session = Session::connect(&own).await?;
        let plan = database::plan::explain(&mut session, engine, &sql, true).await;
        let _ = session.close().await;
        return plan;
    }
    tab.with_session(&params, database, async |s| {
        database::plan::explain(s, engine, &sql, false).await
    })
    .await
}

/// The WHERE clause a set of row filters becomes, for "Show SQL".
#[tauri::command]
pub async fn preview_filter(
    id: String,
    tab_id: String,
    filters: Vec<database::filter::RowFilter>,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let (_, params) = state.connections.query_tab(&id, &tab_id).await?;
    database::filter::where_clause(params.engine, &filters).map(|clause| clause.trim().to_owned())
}

/// Tables whose single-column foreign keys point at this one.
#[tauri::command]
pub async fn list_referencing(
    id: String,
    database: String,
    schema: Option<String>,
    name: String,
    state: State<'_, AppState>,
) -> Result<Vec<database::metadata::Referencing>, AppError> {
    let schema = schema.unwrap_or_default();
    with_session(&state, &id, async |s| {
        s.referencing(&database, &schema, &name).await
    })
    .await
}

/// Applies staged row edits in one transaction on the table tab's session.
/// All or nothing; returns how many statements ran.
#[tauri::command]
pub async fn apply_row_changes(
    id: String,
    tab_id: String,
    target: database::edit::TableTarget,
    changes: Vec<database::edit::RowChange>,
    state: State<'_, AppState>,
) -> Result<usize, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let statements = database::edit::statements(params.engine, &target, &changes)?;
    let result = tab
        .apply(&params, Some(target.database.clone()), &statements)
        .await;
    if let Err(error) = &result
        && is_connection_failure(error)
    {
        state.connections.close_query_tab(&id, &tab_id).await;
        // If it dropped during COMMIT, the server may or may not have saved.
        return Err(AppError::new(
            "connection_lost",
            "The connection was lost while saving. Reconnect and reload the table to see whether the changes were saved.",
        ));
    }
    result
}

/// Largest SQL file opened into the editor; bigger files are run instead.
const OPEN_FILE_LIMIT: u64 = 5 * 1024 * 1024;
/// Largest SQL file Run SQL File reads into memory.
const RUN_FILE_LIMIT: u64 = 256 * 1024 * 1024;

fn read_text_file(path: &str, limit: u64, too_big: &str) -> Result<String, AppError> {
    let unreadable = || AppError::new("file", "That file couldn't be read.");
    let size = std::fs::metadata(path).map_err(|_| unreadable())?.len();
    if size > limit {
        return Err(AppError::new("file_too_large", too_big.to_owned()));
    }
    let bytes = std::fs::read(path).map_err(|_| unreadable())?;
    String::from_utf8(bytes).map_err(|_| {
        AppError::new(
            "file",
            "That file isn't UTF-8 text. Save it as UTF-8 and try again.",
        )
    })
}

/// A SQL file's text, to open in a query tab.
#[tauri::command]
pub async fn read_sql_file(path: String) -> Result<String, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        read_text_file(
            &path,
            OPEN_FILE_LIMIT,
            "That file is too large to edit here. Use Run SQL File… to run it instead.",
        )
    })
    .await
    .map_err(|_| AppError::background_task())?
}

/// Runs a SQL file statement by statement on the tab's session.
#[tauri::command]
pub async fn run_sql_file(
    id: String,
    tab_id: String,
    database: Option<String>,
    path: String,
    keep_going: bool,
    state: State<'_, AppState>,
) -> Result<crate::queries::ScriptSummary, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let engine = params.engine;
    let statements = tauri::async_runtime::spawn_blocking(move || {
        read_text_file(
            &path,
            RUN_FILE_LIMIT,
            "That file is larger than 256 MB. Split it, or use the database's own tools.",
        )
        .map(|text| database::script::split_script(&text, engine))
    })
    .await
    .map_err(|_| AppError::background_task())??;
    tab.run_script(&params, database, &statements, keep_going)
        .await
}

/// Statements done and total while a SQL file runs.
#[tauri::command]
pub async fn sql_file_progress(
    id: String,
    tab_id: String,
    state: State<'_, AppState>,
) -> Result<Option<(usize, usize)>, AppError> {
    Ok(state
        .connections
        .existing_query_tab(&id, &tab_id)
        .await
        .and_then(|(tab, _)| tab.script_progress()))
}

/// How far the tab's running statement has got: rows so far and the first
/// rows to show. `None` when nothing is running.
#[tauri::command]
pub async fn query_progress(
    id: String,
    tab_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::queries::Progress>, AppError> {
    Ok(state
        .connections
        .existing_query_tab(&id, &tab_id)
        .await
        .and_then(|(tab, _)| tab.progress()))
}

/// Runs one user-authored statement, as written, on the query tab's own
/// session. Never retried: if the connection drops, the outcome is unknown
/// and the user decides what to do.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri maps each argument from the frontend.
pub async fn run_statement(
    id: String,
    tab_id: String,
    database: Option<String>,
    sql: String,
    timeout_seconds: Option<u64>,
    max_rows: Option<usize>,
    // Values for the statement's placeholders, bound by the driver. History
    // keeps the statement with its placeholders, never the values.
    bound: Option<Vec<database::values::Param>>,
    state: State<'_, AppState>,
) -> Result<QueryResult, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let max_rows = max_rows
        .unwrap_or(DEFAULT_MAX_ROWS)
        .clamp(1, MAX_ROWS_LIMIT);
    let timeout = timeout_seconds
        .filter(|s| *s > 0)
        .map(std::time::Duration::from_secs);
    let ran_in = database.clone().or_else(|| params.database.clone());
    let started = std::time::Instant::now();
    let result = tab
        .run_with(&params, database, &sql, bound.as_deref(), max_rows, timeout)
        .await;
    record_history(&state, &id, ran_in, &sql, started.elapsed(), &result).await;
    if let Err(error) = &result
        && is_connection_failure(error)
    {
        // The tab's session is unusable; open a new one next time.
        state.connections.close_query_tab(&id, &tab_id).await;
        return Err(database::error::connection_lost());
    }
    result
}

/// Adds a run to query history (if enabled). Failing to record never fails
/// the query itself.
async fn record_history(
    state: &AppState,
    profile_id: &str,
    database: Option<String>,
    sql: &str,
    elapsed: std::time::Duration,
    result: &Result<QueryResult, AppError>,
) {
    let (status, row_count, error) = match result {
        Ok(r) if !r.columns.is_empty() => ("completed", Some(r.total_rows as i64), None),
        Ok(r) => ("completed", r.rows_affected.map(|n| n as i64), None),
        Err(e) if e.kind == "query_cancelled" => ("cancelled", None, Some(e.message.clone())),
        Err(e) if e.kind == "query_timed_out" => ("timed_out", None, Some(e.message.clone())),
        Err(e) => (
            "failed",
            None,
            Some(e.detail.clone().unwrap_or_else(|| e.message.clone())),
        ),
    };
    let profile_id = profile_id.to_owned();
    let sql = sql.to_owned();
    let _ = state
        .blocking(move |connection, _| {
            let name: String = connection
                .query_row(
                    "SELECT name FROM connection_profiles WHERE id = ?1",
                    [&profile_id],
                    |row| row.get(0),
                )
                .unwrap_or_default();
            crate::library::record(
                connection,
                &crate::library::NewHistoryEntry {
                    profile_id: &profile_id,
                    connection_name: &name,
                    database_name: database.as_deref().unwrap_or_default(),
                    query_text: &sql,
                    duration_ms: elapsed.as_millis() as i64,
                    status,
                    row_count,
                    error_summary: error.as_deref(),
                },
            )
        })
        .await;
}

/// Stops the statement running in a query tab. Returns false if nothing ran.
#[tauri::command]
pub async fn cancel_statement(
    id: String,
    tab_id: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    Ok(
        match state.connections.existing_query_tab(&id, &tab_id).await {
            Some((tab, params)) => tab.cancel(&params),
            None => false,
        },
    )
}

/// Closes a query tab's session (when the tab closes).
#[tauri::command]
pub async fn close_query_tab(
    id: String,
    tab_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.connections.close_query_tab(&id, &tab_id).await;
    Ok(())
}

/// Trusts an SSH server's host key after the user checked its fingerprint.
/// Returns the stored fingerprint.
#[tauri::command]
pub async fn trust_ssh_host_key(
    host: String,
    port: u16,
    public_key: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    state
        .blocking(move |connection, _| {
            crate::ssh::trust_host_key(connection, &host, port, &public_key)
        })
        .await
}

/// What `~/.ssh/config` says about a `Host` alias, and where it was read from.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedSshAlias {
    config: crate::ssh::config::SshHostConfig,
    source: String,
}

/// Looks up an SSH `Host` alias in the user's own config. Returns null when
/// the alias isn't there, or when the config can't be read; the user is never
/// stopped from typing a host by hand.
#[tauri::command]
pub async fn resolve_ssh_alias(alias: String) -> Option<ResolvedSshAlias> {
    let config = crate::ssh::config::resolve_user_alias(&alias)?;
    Some(ResolvedSshAlias {
        config,
        source: crate::ssh::config::user_config_label(),
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgetHostKeysResult {
    /// ~/.ssh/known_hosts still has a key for this server; only the user
    /// can remove it there (ssh-keygen -R).
    still_in_openssh_known_hosts: bool,
}

/// Forgets the SSH host keys Thabi Studio saved for a server.
#[tauri::command]
pub async fn forget_ssh_host_keys(
    host: String,
    port: u16,
    state: State<'_, AppState>,
) -> Result<ForgetHostKeysResult, AppError> {
    state
        .blocking(move |connection, _| {
            Ok(ForgetHostKeysResult {
                still_in_openssh_known_hosts: crate::ssh::forget_host_keys(
                    connection, &host, port,
                )?,
            })
        })
        .await
}

async fn stored_result(
    state: &AppState,
    id: &str,
    tab_id: &str,
    result_id: &str,
) -> Result<std::sync::Arc<crate::results::StoredResult>, AppError> {
    let (tab, _) = state
        .connections
        .existing_query_tab(id, tab_id)
        .await
        .ok_or_else(|| {
            AppError::new(
                "result_expired",
                "This result is no longer kept. Run the query again.",
            )
        })?;
    tab.result(result_id)
}

/// A page of a kept result, in the grid's current sort order.
#[tauri::command]
pub async fn fetch_rows(
    id: String,
    tab_id: String,
    result_id: String,
    offset: usize,
    limit: usize,
    sort: Option<crate::results::Sort>,
    state: State<'_, AppState>,
) -> Result<Vec<Vec<database::Cell>>, AppError> {
    let result = stored_result(&state, &id, &tab_id, &result_id).await?;
    tauri::async_runtime::spawn_blocking(move || result.page(offset, limit, sort))
        .await
        .map_err(|_| AppError::background_task())
}

/// Where a kept result's SQL formats point: the table it was read from when
/// known, otherwise a placeholder name the user replaces.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultTarget {
    #[serde(default)]
    pub database: String,
    #[serde(default)]
    pub schema: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub primary_key: Vec<String>,
}

fn result_table(engine: database::Engine, target: &ResultTarget) -> String {
    let name = if target.name.is_empty() {
        "query_result"
    } else {
        &target.name
    };
    database::quote::table(engine, &target.database, &target.schema, name)
}

/// Selected cells for the clipboard, in any transfer format.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri maps each argument from the frontend.
pub async fn copy_rows(
    id: String,
    tab_id: String,
    result_id: String,
    scope: crate::results::Scope,
    sort: Option<crate::results::Sort>,
    format: database::transfer::TransferFormat,
    target: Option<ResultTarget>,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    use database::transfer::TransferFormat;
    let result = stored_result(&state, &id, &tab_id, &result_id).await?;
    let engine = state.connections.query_tab(&id, &tab_id).await?.1.engine;
    let target = target.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || match format {
        TransferFormat::Tsv => Ok(result.to_text(&scope, sort, crate::results::TextFormat::Tsv)),
        TransferFormat::Csv => Ok(result.to_text(&scope, sort, crate::results::TextFormat::Csv)),
        other => {
            let mut out = Vec::new();
            result.write_rows(
                &mut out,
                &scope,
                sort,
                other,
                engine,
                result_table(engine, &target),
                &target.primary_key,
            )?;
            Ok(String::from_utf8(out).unwrap_or_default())
        }
    })
    .await
    .map_err(|_| AppError::background_task())?
}

/// Writes a kept result to a file in the chosen format.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri maps each argument from the frontend.
pub async fn export_result(
    id: String,
    tab_id: String,
    result_id: String,
    path: String,
    scope: crate::results::Scope,
    sort: Option<crate::results::Sort>,
    format: database::transfer::TransferFormat,
    target: Option<ResultTarget>,
    state: State<'_, AppState>,
) -> Result<crate::results::ExportSummary, AppError> {
    use database::transfer::TransferFormat;
    let result = stored_result(&state, &id, &tab_id, &result_id).await?;
    let engine = state.connections.query_tab(&id, &tab_id).await?.1.engine;
    let target = target.unwrap_or_default();
    let path = match format {
        TransferFormat::Csv => crate::results::export_path(&path)?,
        other => results_extension(&path, other),
    };
    tauri::async_runtime::spawn_blocking(move || {
        if format == TransferFormat::Xlsx {
            let rows = result.export_xlsx(&path, &scope, sort)?;
            return Ok(crate::results::ExportSummary {
                rows,
                path: path.to_string_lossy().into_owned(),
            });
        }
        if format == TransferFormat::Csv {
            let rows = result.export_csv(&path, &scope, sort)?;
            return Ok(crate::results::ExportSummary {
                rows,
                path: path.to_string_lossy().into_owned(),
            });
        }
        use std::io::Write;
        let file = std::fs::File::create(&path).map_err(|_| {
            AppError::new(
                "export_failed",
                "That file couldn't be created. Check the folder and try again.",
            )
        })?;
        let mut out = std::io::BufWriter::new(file);
        let rows = result.write_rows(
            &mut out,
            &scope,
            sort,
            format,
            engine,
            result_table(engine, &target),
            &target.primary_key,
        )?;
        out.flush().ok();
        Ok(crate::results::ExportSummary {
            rows,
            path: path.to_string_lossy().into_owned(),
        })
    })
    .await
    .map_err(|_| AppError::background_task())?
}

/// Writes a kept result to a CSV file the user chose in the save dialog.
#[tauri::command]
pub async fn export_csv(
    id: String,
    tab_id: String,
    result_id: String,
    path: String,
    scope: crate::results::Scope,
    sort: Option<crate::results::Sort>,
    state: State<'_, AppState>,
) -> Result<crate::results::ExportSummary, AppError> {
    let result = stored_result(&state, &id, &tab_id, &result_id).await?;
    let path = crate::results::export_path(&path)?;
    tauri::async_runtime::spawn_blocking(move || {
        let rows = result.export_csv(&path, &scope, sort)?;
        Ok(crate::results::ExportSummary {
            rows,
            path: path.to_string_lossy().into_owned(),
        })
    })
    .await
    .map_err(|_| AppError::background_task())?
}

/// Copies from a table, re-reading it from the server.
///
/// Capped at the connection's row limit: the clipboard is not a place to put a
/// million rows, and a silent truncation would be worse than a number.
#[tauri::command]
pub async fn copy_table(
    id: String,
    tab_id: String,
    request: database::transfer::TransferRequest,
    format: database::transfer::TransferFormat,
    max_rows: Option<usize>,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let limit = max_rows
        .unwrap_or(DEFAULT_MAX_ROWS)
        .clamp(1, MAX_ROWS_LIMIT);
    let mut text: Vec<u8> = Vec::new();
    database::transfer::stream_table(&tab, &params, &request, format, Some(limit), &mut text)
        .await?;
    Ok(String::from_utf8(text).unwrap_or_default())
}

/// Writes a schema, or chosen tables, to a `.sql` file the same engine can run.
///
/// A backup does not need an open query tab, so this runs off the connection's
/// own session rather than a tab's. Progress is emitted as it goes — a dump of
/// a real database is minutes, and a button that does nothing visible for two
/// of them looks broken.
#[tauri::command]
pub async fn export_sql(
    app: tauri::AppHandle,
    id: String,
    database: String,
    schema: Option<String>,
    options: database::dump::DumpOptions,
    path: String,
    state: State<'_, AppState>,
) -> Result<database::dump::DumpSummary, AppError> {
    use database::dump::DumpSummary;

    if !options.wants_anything() {
        return Err(AppError::new(
            "nothing_to_export",
            "Choose structure, data, or both.",
        ));
    }
    let schema = schema.unwrap_or_default();

    let engine = with_session(&state, &id, async |session| {
        Ok(session.server_info().engine)
    })
    .await?;
    // Every structure is read before anything is written, so a table that
    // cannot be described stops the export rather than leaving a hole in a
    // file that looks complete.
    let planned = with_session(&state, &id, async |session| {
        database::dump::plan(session, &database, &schema, &options).await
    })
    .await?;
    if planned.is_empty() {
        return Err(AppError::new(
            "nothing_to_export",
            "There are no tables here to export.",
        ));
    }
    let session = state.connections.session(&id).await?;
    let handle = tokio::runtime::Handle::current();
    let cancel = state.start_export(&id);
    let total = planned.len();

    let result = tauri::async_runtime::spawn_blocking(move || {
        use std::io::Write;
        let file = std::fs::File::create(&path).map_err(|_| {
            AppError::new(
                "export_failed",
                "That file couldn't be created. Check the folder and try again.",
            )
        })?;
        let mut writer = database::dump::CountingWriter::new(std::io::BufWriter::new(file));
        let banner = format!(
            "{}{}{}\n",
            database::dump::header(engine, &database, &schema, &generated_stamp()),
            database::dump::contents_note(&options),
            database::dump::fidelity_note(engine, &options).unwrap_or_default(),
        );
        writer.write_all(banner.as_bytes()).map_err(wump)?;

        let mut rows = 0u64;
        let mut read = 0usize;
        for (name, structure) in &planned {
            read += 1;
            // Cloned per call rather than moved, so the closure is FnMut and
            // can be called once per page for as long as the table takes.
            let table = name.clone();
            let app = &app;
            let mut announce = |rows: u64| {
                let _ = app.emit(
                    "sql-export-progress",
                    database::dump::DumpProgress {
                        table: table.clone(),
                        tables: total,
                        rows,
                    },
                );
            };
            announce(0);
            if options.structure {
                for statement in database::dump::structure_statements(
                    engine, &database, &schema, name, structure,
                ) {
                    writer.write_all(statement.as_bytes()).map_err(wump)?;
                }
            }
            if options.data {
                // The lock is taken and released around one table at a time
                // rather than held for the whole dump, so a query on the same
                // connection is not blocked behind an export.
                let mut guard = handle.block_on(session.lock());
                let live = guard.as_mut().ok_or_else(AppError::not_connected)?;
                let target = database::ddl::TableTarget {
                    database: database.clone(),
                    schema: if schema.is_empty() {
                        None
                    } else {
                        Some(schema.clone())
                    },
                    name: name.clone(),
                };
                let mut run =
                    database::dump::WriteRun::new(options.row_limit, cancel.clone(), &mut announce);
                rows += handle.block_on(database::dump::write_table_rows(
                    live,
                    &target,
                    structure,
                    &mut run,
                    &mut writer,
                ))?;
            }
        }
        writer.flush().map_err(wump)?;
        let _ = read;
        Ok(DumpSummary {
            tables: total,
            rows,
            bytes: writer.bytes(),
        })
    })
    .await;

    // Cleared whichever way it ended. A flag left behind makes the Stop button
    // claim there is something to stop on the next export, when there is not.
    state.finish_export(&id);
    result.map_err(|_| AppError::background_task())?
}

/// Stops a running dump. False when nothing is running.
#[tauri::command]
pub fn cancel_sql_export(id: String, state: State<'_, AppState>) -> bool {
    state.cancel_export(&id)
}

/// A timestamp for the file's header, taken from the clock rather than the
/// server, because a backup labelled only "on export" cannot be told from a
/// weekly one.
fn generated_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or(0);
    // ISO 8601 in UTC, computed rather than formatted: a date library is a
    // dependency for one comment line.
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    let time = seconds % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        time / 3600,
        (time % 3600) / 60,
        time % 60
    )
}

/// Days since 1970 to a calendar date, by Howard Hinnant's algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// A write failure inside the blocking writer, where an io::Error has to become
/// an AppError to travel back out.
fn wump(cause: std::io::Error) -> AppError {
    AppError::new("write_failed", "Couldn't write the file.").with_detail(Some(cause.to_string()))
}

/// Exports a table, streaming it straight to the chosen file.
///
/// The extension decides the format, and the extension is added when it is
/// missing, so a file always opens as what it claims to be.
#[tauri::command]
pub async fn export_table(
    id: String,
    tab_id: String,
    request: database::transfer::TransferRequest,
    format: database::transfer::TransferFormat,
    path: String,
    state: State<'_, AppState>,
) -> Result<crate::results::ExportSummary, AppError> {
    use database::transfer::TransferFormat;
    let path = match format {
        TransferFormat::Csv => crate::results::export_path(&path)?,
        other => results_extension(&path, other),
    };
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    if format == TransferFormat::Xlsx {
        let written = database::transfer::export_table_xlsx(&tab, &params, &request, &path).await?;
        return Ok(crate::results::ExportSummary {
            rows: written.rows,
            path: path.to_string_lossy().into_owned(),
        });
    }
    // Writing a file is blocking work, so it runs off the async workers. This
    // is a blocking pool thread, where blocking on the runtime is allowed.
    let handle = tokio::runtime::Handle::current();
    let destination = path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        use std::io::Write;
        let file = std::fs::File::create(&path).map_err(|_| {
            AppError::new(
                "export_failed",
                "That file couldn't be created. Check the folder and try again.",
            )
        })?;
        let mut writer = std::io::BufWriter::new(file);
        let written = handle.block_on(database::transfer::stream_table(
            &tab,
            &params,
            &request,
            format,
            None,
            &mut writer,
        ))?;
        writer.flush().ok();
        Ok(crate::results::ExportSummary {
            rows: written.rows,
            path: destination.to_string_lossy().into_owned(),
        })
    })
    .await
    .map_err(|_| AppError::background_task())?
}

/// The extension each format should end up with.
fn results_extension(path: &str, format: database::transfer::TransferFormat) -> std::path::PathBuf {
    use database::transfer::TransferFormat;
    use std::path::PathBuf;
    let wanted = match format {
        TransferFormat::Csv | TransferFormat::Tsv => "csv",
        TransferFormat::Json => "json",
        TransferFormat::Markdown => "md",
        TransferFormat::Xlsx => "xlsx",
        TransferFormat::SqlInsert | TransferFormat::SqlUpdate => "sql",
    };
    let path = PathBuf::from(path);
    // TSV and CSV both end in .csv on purpose: spreadsheets open either.
    let already = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .is_some_and(|e| matches!(e.as_str(), "csv" | "tsv" | "json" | "md" | "sql" | "xlsx"));
    if already {
        path
    } else {
        path.with_extension(wanted)
    }
}

/// The DDL for a table's staged changes, and how they will be applied.
///
/// Pure text: nothing runs and no connection is touched, so the review dialog
/// has something to show before anything is sent.
#[tauri::command]
pub async fn preview_table_ddl(
    id: String,
    tab_id: String,
    target: database::ddl::TableTarget,
    plan: database::ddl::TablePlan,
    state: State<'_, AppState>,
) -> Result<database::ddl::DdlPreview, AppError> {
    let (_, params) = state.connections.query_tab(&id, &tab_id).await?;
    Ok(database::ddl::preview(
        params.engine,
        &target.database,
        target.schema.as_deref().unwrap_or_default(),
        &target.name,
        &plan,
    ))
}

/// Applies a table's staged changes.
///
/// The script is built by the same call the preview used, so the text that was
/// read is the text that runs.
#[tauri::command]
pub async fn apply_table_ddl(
    id: String,
    tab_id: String,
    target: database::ddl::TableTarget,
    plan: database::ddl::TablePlan,
    state: State<'_, AppState>,
) -> Result<usize, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let script = database::ddl::table_script(
        params.engine,
        &target.database,
        target.schema.as_deref().unwrap_or_default(),
        &target.name,
        &plan,
    );
    tab.apply(&params, Some(target.database.clone()), &script)
        .await
}

/// The statements for a whole-table action, reading the structure a copy is
/// made from. Both commands come through here, so the SQL shown is the SQL
/// that runs.
async fn action_script_for(
    tab: &crate::queries::QueryTab,
    params: &database::ConnectParams,
    engine: database::Engine,
    target: &database::ddl::TableTarget,
    action: &database::ddl::TableAction,
) -> Result<Vec<String>, AppError> {
    let schema = target.schema.as_deref().unwrap_or_default();
    let needs_structure = matches!(
        action,
        database::ddl::TableAction::DuplicateStructure { .. }
            | database::ddl::TableAction::DuplicateWithData { .. }
    );
    let structure = if needs_structure {
        let found = tab
            .with_session(params, Some(target.database.clone()), async |session| {
                session
                    .describe(
                        &target.database,
                        schema,
                        &target.name,
                        database::ObjectKind::Table,
                    )
                    .await
            })
            .await?;
        Some(database::ddl::spec_from_structure(&found))
    } else {
        None
    };
    database::ddl::action_script(
        engine,
        &target.database,
        schema,
        &target.name,
        action,
        structure.as_ref(),
    )
    .map_err(|message| AppError::new("invalid", message))
}

/// The SQL for a whole-table action, before it runs.
///
/// The same two-step as a staged change: read the SQL, then run it. An action
/// that skipped the first step would be the one here that cannot be taken back.
#[tauri::command]
pub async fn preview_table_action(
    id: String,
    tab_id: String,
    target: database::ddl::TableTarget,
    action: database::ddl::TableAction,
    state: State<'_, AppState>,
) -> Result<database::ddl::DdlPreview, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let script = action_script_for(&tab, &params, params.engine, &target, &action).await?;
    Ok(database::ddl::preview_of(params.engine, script))
}

/// Runs a whole-table action: the same script the preview showed.
#[tauri::command]
pub async fn apply_table_action(
    id: String,
    tab_id: String,
    target: database::ddl::TableTarget,
    action: database::ddl::TableAction,
    state: State<'_, AppState>,
) -> Result<usize, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let script = action_script_for(&tab, &params, params.engine, &target, &action).await?;
    tab.apply(&params, Some(target.database.clone()), &script)
        .await
}

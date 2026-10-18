import { call } from "./ipc.js";

/** @returns {Promise<{ name: string, version: string }>} */
export function getAppInfo() {
  return call("app_info");
}

export function getAppSettings() {
  return call("get_app_settings");
}

/** The app password lock (docs/decisions/0033-app-password.md). */
export function appLockStatus() {
  return call("app_lock_status");
}

export function unlockApp(password) {
  return call("unlock_app", { password });
}

export function lockApp() {
  return call("lock_app");
}

export function setAppPassword(current, password) {
  return call("set_app_password", { current, password });
}

export function removeAppPassword(current) {
  return call("remove_app_password", { current });
}

export function resetAppPassword() {
  return call("reset_app_password");
}

/** Ways back in when the app password is forgotten. */
export function recoveryOptions() {
  return call("recovery_options");
}

export function recoverWithCode(code, password) {
  return call("recover_with_code", { code, password });
}

export function recoverWithAnswers(answers, password) {
  return call("recover_with_answers", { answers, password });
}

export function regenerateRecoveryCodes(current) {
  return call("regenerate_recovery_codes", { current });
}

export function setSecurityQuestions(current, questions) {
  return call("set_security_questions", { current, questions });
}

export function removeSecurityQuestions(current) {
  return call("remove_security_questions", { current });
}

export function saveRecoveryCodes(path, codes) {
  return call("save_recovery_codes", { path, codes });
}

export function importLegacyPreferences(preferences) {
  return call("import_legacy_preferences", { preferences });
}

export function setAppSetting(key, value) {
  return call("set_app_setting", { key, value });
}

export function listConnectionProfiles() {
  return call("list_connection_profiles");
}

export function createConnectionProfile(input) {
  return call("create_connection_profile", { input });
}

export function updateConnectionProfile(id, input) {
  return call("update_connection_profile", { id, input });
}

export function duplicateConnectionProfile(id) {
  return call("duplicate_connection_profile", { id });
}

export function deleteConnectionProfile(id) {
  return call("delete_connection_profile", { id });
}

export function selectConnectionProfile(id) {
  return call("select_connection_profile", { id });
}

/** Saves the order the user dragged connections into; returns the new list. */
export function reorderConnectionProfiles(ids) {
  return call("reorder_connection_profiles", { ids });
}

/**
 * What ~/.ssh/config says about a `Host` alias: { config, source }, or null.
 * Only an exact alias match resolves; wildcards and Include are not followed.
 */
export function resolveSshAlias(alias) {
  return call("resolve_ssh_alias", { alias });
}

/** Tests unsaved form values; `profileId` lets an edited profile reuse its saved password. */
export function testConnectionInput(input, profileId = null) {
  return call("test_connection_input", { input, profileId });
}

/** `database` opens another database for this connection only. */
/**
 * `secrets` are typed for this one connection when it asks every time:
 * { databasePassword?, sshPassword?, sshKeyPassphrase? }. Never saved.
 */
export function connectConnectionProfile(id, database = null, secrets = null) {
  return call("connect_connection_profile", { id, database, secrets });
}

export function disconnectConnectionProfile(id) {
  return call("disconnect_connection_profile", { id });
}

/** @returns {Promise<Array<{ profileId: string, connectionId: string, state: "connecting" | "connected" | "closing", server: { engine: string, version: string } | null }>>} */
export function listConnections() {
  return call("list_connections");
}

export function listDatabases(id) {
  return call("list_databases", { id });
}

/**
 * Runs one statement on a query tab's own session. Keeps at most 1,000 rows.
 * Rejects with kind "query_cancelled" or "query_timed_out" when stopped.
 */
export function runStatement(
  id,
  tabId,
  { database, sql, timeoutSeconds, maxRows, bound = null },
) {
  return call("run_statement", {
    id,
    tabId,
    database,
    sql,
    timeoutSeconds,
    maxRows,
    bound,
  });
}

/** A page of a kept result in the given sort ({ column, descending }). */
export function fetchRows(id, tabId, resultId, { offset, limit, sort }) {
  return call("fetch_rows", { id, tabId, resultId, offset, limit, sort });
}

/**
 * Cells as text for the clipboard. scope: { rows?: [start, end],
 * columns?: number[], includeHeader }. format: "tsv" | "csv".
 */
/**
 * Copies kept result rows in any transfer format. `target` names the table
 * SQL formats address ({ database, schema, name, primaryKey }), when known.
 */
export function copyRows(id, tabId, resultId, { scope, sort, format, target }) {
  return call("copy_rows", {
    id,
    tabId,
    resultId,
    scope,
    sort,
    format,
    target: target ?? null,
  });
}

/** Writes kept result rows to a file in any transfer format. */
export function exportResult(
  id,
  tabId,
  resultId,
  { path, scope, sort, format, target },
) {
  return call("export_result", {
    id,
    tabId,
    resultId,
    path,
    scope,
    sort,
    format,
    target: target ?? null,
  });
}

/** @returns {Promise<{ rows: number, path: string }>} */
export function exportCsv(id, tabId, resultId, { path, scope, sort }) {
  return call("export_csv", { id, tabId, resultId, path, scope, sort });
}

/** @returns {Promise<boolean>} false when nothing was running. */
export function cancelStatement(id, tabId) {
  return call("cancel_statement", { id, tabId });
}

export function closeQueryTab(id, tabId) {
  return call("close_query_tab", { id, tabId });
}

/** Trusts an SSH server's host key after the user checked its fingerprint. */
export function trustSshHostKey(host, port, publicKey) {
  return call("trust_ssh_host_key", { host, port, publicKey });
}

/** @returns {Promise<{ stillInOpensshKnownHosts: boolean }>} */
export function forgetSshHostKeys(host, port) {
  return call("forget_ssh_host_keys", { host, port });
}

/** @returns {Promise<string[] | null>} null when the engine has no schemas (MySQL). */
export function listSchemas(id, database) {
  return call("list_schemas", { id, database });
}

/** @returns {Promise<Array<{ name: string, kind: "table" | "view" | "materialized_view" | "foreign_table" }>>} */
export function listObjects(id, database, schema = null) {
  return call("list_objects", { id, database, schema });
}

/**
 * Everything the ER diagram draws for one schema.
 *
 * @returns {Promise<{ tables: Array<{ name: string, columns: Array<{ name: string, dataType: string, nullable: boolean, primaryKey: boolean }> }>, foreignKeys: Array<{ name: string, fromTable: string, fromColumns: string[], toTable: string, toColumns: string[] }> }>}
 */
export function schemaDiagram(id, database, schema = null) {
  return call("schema_diagram", { id, database, schema });
}

export function describeObject(id, { database, schema, name, kind }) {
  return call("describe_object", { id, database, schema, name, kind });
}

/**
 * Reads a table or view into the object tab's own session. The result pages,
 * sorts and exports like a query result. Bounded by `maxRows`.
 */
/**
 * Staged row edits on a table tab. target: { database, schema, name };
 * changes: [{ kind: "update", key, set } | { kind: "delete", key } |
 * { kind: "insert", values }], each key/set/values a [{ column, value }].
 * @returns {Promise<string[]>} the statements, for review
 */
export function previewRowChanges(id, tabId, target, changes) {
  return call("preview_row_changes", { id, tabId, target, changes });
}

/**
 * Values a foreign key column may take. reference: { schema, table, column }.
 * @returns {Promise<{ value: string, label: string | null }[]>}
 */
export function lookupReference(id, tabId, database, reference, search) {
  return call("lookup_reference", { id, tabId, database, reference, search });
}

/** The WHERE clause row filters become, for display. */
export function previewFilter(id, tabId, filters) {
  return call("preview_filter", { id, tabId, filters });
}

/**
 * Single-column foreign keys elsewhere that point at a table.
 * @returns {Promise<{ schema: string, table: string, column: string, referencedColumn: string }[]>}
 */
export function listReferencing(id, { database, schema, name }) {
  return call("list_referencing", { id, database, schema, name });
}

/** Applies staged row edits in one transaction. @returns {Promise<number>} */
export function applyRowChanges(id, tabId, target, changes) {
  return call("apply_row_changes", { id, tabId, target, changes });
}

/**
 * One page of a table, read from the server (Phase 12).
 * `filters`: [{ column, op, value }], AND-ed; op "raw" takes a SQL condition.
 * `primaryKey`: the table's key, so pages have a stable order.
 * `order`: { column, descending } to sort on the server.
 */
export function openObjectData(
  id,
  tabId,
  {
    database,
    schema,
    name,
    maxRows,
    filters = [],
    primaryKey = [],
    order = null,
  },
) {
  return call("open_object_data", {
    id,
    tabId,
    database,
    schema,
    name,
    maxRows,
    filters,
    primaryKey,
    order,
  });
}

/**
 * The DDL for a table's staged changes, and what happens if one of them fails.
 * Runs nothing: the review dialog shows this before anything is sent.
 */
export function previewTableDdl(id, tabId, target, plan) {
  return call("preview_table_ddl", { id, tabId, target, plan });
}

/** Applies a table's staged changes. The script is the one previewed. */
export function applyTableDdl(id, tabId, target, plan) {
  return call("apply_table_ddl", { id, tabId, target, plan });
}

/** The SQL for a whole-table action, before it runs. */
export function previewTableAction(id, tabId, target, action) {
  return call("preview_table_action", { id, tabId, target, action });
}

/** Runs a whole-table action: the same script the preview showed. */
export function applyTableAction(id, tabId, target, action) {
  return call("apply_table_action", { id, tabId, target, action });
}

/** One page of a table at `offset`, sorted on the server. */
export function fetchTablePage(
  id,
  tabId,
  { database, schema, name, filters = [], primaryKey = [], order = null },
  { offset, limit },
) {
  return call("fetch_table_page", {
    id,
    tabId,
    database,
    schema,
    name,
    filters,
    primaryKey,
    order,
    offset,
    limit,
  });
}

/**
 * Copies from a table, re-reading it from the server.
 * `format`: "csv" | "tsv" | "json" | "markdown" | "sqlInsert" | "sqlUpdate".
 */
export function copyTable(
  id,
  tabId,
  {
    database,
    schema,
    name,
    filters = [],
    primaryKey = [],
    columns = [],
    order = null,
  },
  { format, maxRows = null, scope = {} } = {},
) {
  return call("copy_table", {
    id,
    tabId,
    request: {
      database,
      schema,
      name,
      filters,
      primaryKey,
      order,
      columns,
      columnsScope: scope.columns ?? [],
      rows: scope.rows ?? null,
      includeHeader: scope.includeHeader ?? true,
    },
    format,
    maxRows,
  });
}

/**
 * Writes a schema, or chosen tables, to a `.sql` file the same engine can run.
 *
 * `options`: { structure, data, tables: string[], rowLimit?: number }. An
 * empty `tables` means every table in the schema.
 *
 * @returns {Promise<{ tables: number, rows: number, bytes: number }>}
 */
export function exportSql(id, { database, schema, options, path }) {
  return call("export_sql", { id, database, schema, options, path });
}

/** Stops a running SQL dump. False when nothing is running. */
export function cancelSqlExport(id) {
  return call("cancel_sql_export", { id });
}

/** Exports a table to a file, streaming from the server. */
export function exportTable(
  id,
  tabId,
  {
    database,
    schema,
    name,
    filters = [],
    primaryKey = [],
    columns = [],
    order = null,
  },
  { format, path, scope = {} } = {},
) {
  return call("export_table", {
    id,
    tabId,
    request: {
      database,
      schema,
      name,
      filters,
      primaryKey,
      order,
      columns,
      columnsScope: scope.columns ?? [],
      rows: scope.rows ?? null,
      includeHeader: scope.includeHeader ?? true,
    },
    format,
    path,
  });
}

/** The exact number of rows, when the catalog estimate isn't trusted. */
export function countTableRows(
  id,
  tabId,
  { database, schema, name, filters = [] },
) {
  return call("count_table_rows", {
    id,
    tabId,
    database,
    schema,
    name,
    filters,
  });
}

export function listHistory({ search = "", limit = 100, offset = 0 } = {}) {
  return call("list_history", { search, limit, offset });
}

export function deleteHistoryEntry(id) {
  return call("delete_history_entry", { id });
}

/** @returns {Promise<number>} entries removed */
export function clearHistory() {
  return call("clear_history");
}

export function listSavedQueries() {
  return call("list_saved_queries");
}

/** input: { id?, name, queryText, profileId?, notes?, folder? } */
export function saveQuery(input) {
  return call("save_query", { input });
}

export function deleteSavedQuery(id) {
  return call("delete_saved_query", { id });
}

/** The filter presets saved for one table. */
export function listFilterPresets(
  profileId,
  databaseName,
  schemaName,
  objectName,
) {
  return call("list_filter_presets", {
    profileId,
    databaseName,
    schemaName,
    objectName,
  });
}

/**
 * Saves a filter preset, or updates the one `id` names. Saving over a name
 * already used on the same table updates that preset.
 * input: { id?, profileId, databaseName, schemaName, objectName, name, filters }
 */
export function saveFilterPreset(input) {
  return call("save_filter_preset", { input });
}

export function deleteFilterPreset(id) {
  return call("delete_filter_preset", { id });
}

/** Renames (or, with to = "", removes) a saved-query folder. @returns {Promise<number>} */
export function renameSavedFolder(from, to) {
  return call("rename_saved_folder", { from, to });
}

export function saveWorkspaceTabs(tabs) {
  return call("save_workspace_tabs", { tabs });
}

export function loadWorkspaceTabs() {
  return call("load_workspace_tabs");
}

/** Writes every connection, without secrets, to `path`. @returns {Promise<number>} */
export function exportConnectionProfiles(path) {
  return call("export_connection_profiles", { path });
}

/**
 * Imports connections from an export file; secrets are never imported.
 * @returns {Promise<{ imported: number, skipped: { name: string, reason: string }[] }>}
 */
export function importConnectionProfiles(path) {
  return call("import_connection_profiles", { path });
}

/** Opens another workspace window (File > New Window). */
export function openWorkspaceWindow() {
  return call("open_workspace_window");
}

/**
 * Tables and columns of the tab's database, for autocomplete.
 * @returns {Promise<{ schema: string, table: string, columns: string[] }[]>}
 */
export function completionSchema(id, tabId, database) {
  return call("completion_schema", { id, tabId, database: database ?? null });
}

/**
 * The plan for one statement. `analyze` (PostgreSQL) runs it for real
 * inside a transaction that is rolled back.
 * @returns {Promise<{ nodes: object[], raw: string, analyzed: boolean }>}
 */
export function explainStatement(
  id,
  tabId,
  { database, sql, analyze = false },
) {
  return call("explain_statement", {
    id,
    tabId,
    database: database ?? null,
    sql,
    analyze,
  });
}

/**
 * A running statement's progress: rows so far and its first rows.
 * @returns {Promise<{ received: number, preview: object | null } | null>}
 */
export function queryProgress(id, tabId) {
  return call("query_progress", { id, tabId });
}

/** A SQL file's text (up to 5 MB), to open in a tab. */
export function readSqlFile(path) {
  return call("read_sql_file", { path });
}

/**
 * Runs a SQL file statement by statement on the tab's session.
 * @returns {Promise<{ total: number, ran: number, rowsAffected: number, cancelled: boolean, errors: { line: number, message: string, detail?: string }[] }>}
 */
export function runSqlFile(id, tabId, { database, path, keepGoing = false }) {
  return call("run_sql_file", {
    id,
    tabId,
    database: database ?? null,
    path,
    keepGoing,
  });
}

/** [done, total] while a SQL file runs, or null. */
export function sqlFileProgress(id, tabId) {
  return call("sql_file_progress", { id, tabId });
}

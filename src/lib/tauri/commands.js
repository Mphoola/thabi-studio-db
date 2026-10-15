import { call } from "./ipc.js";

/** @returns {Promise<{ name: string, version: string }>} */
export function getAppInfo() {
  return call("app_info");
}

export function getAppSettings() {
  return call("get_app_settings");
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
  { database, sql, timeoutSeconds, maxRows },
) {
  return call("run_statement", {
    id,
    tabId,
    database,
    sql,
    timeoutSeconds,
    maxRows,
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
export function copyRows(id, tabId, resultId, { scope, sort, format }) {
  return call("copy_rows", { id, tabId, resultId, scope, sort, format });
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

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

/** Tests unsaved form values; `profileId` lets an edited profile reuse its saved password. */
export function testConnectionInput(input, profileId = null) {
  return call("test_connection_input", { input, profileId });
}

/** `database` opens another database for this connection only. */
export function connectConnectionProfile(id, database = null) {
  return call("connect_connection_profile", { id, database });
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

/** Runs SQL on the profile's session. Keeps at most 1,000 rows. */
export function runQuery(id, sql) {
  return call("run_query", { id, sql });
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

/** First 100 rows of a table or view. */
export function previewObject(id, { database, schema, name }) {
  return call("preview_object", { id, database, schema, name });
}

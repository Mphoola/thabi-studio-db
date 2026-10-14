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

export function connectConnectionProfile(id) {
  return call("connect_connection_profile", { id });
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

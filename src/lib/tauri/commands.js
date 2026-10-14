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

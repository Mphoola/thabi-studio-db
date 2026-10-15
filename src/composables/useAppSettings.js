import { readonly, ref } from "vue";
import { setAppSetting } from "../lib/tauri/commands.js";
import { isTauri } from "../lib/tauri/ipc.js";
import { toUserMessage } from "../lib/errors.js";
import { useToasts } from "./useToasts.js";

const defaults = {
  themeMode: "system",
  sidebarWidth: 240,
  sidebarOpen: true,
  selectedProfileId: null,
  queryTimeoutSeconds: 0,
  maxResultRows: 10000,
};
const nativeKeys = {
  themeMode: "theme_mode",
  sidebarWidth: "sidebar_width",
  sidebarOpen: "sidebar_open",
  queryTimeoutSeconds: "query_timeout_seconds",
  maxResultRows: "max_result_rows",
};
const settings = ref({ ...defaults });

export function initializeAppSettings(value) {
  settings.value = { ...defaults, ...value };
}

export function applyLocalAppSetting(key, value) {
  settings.value = { ...settings.value, [key]: value };
}

export function persistAppSetting(key, value) {
  applyLocalAppSetting(key, value);
  if (!isTauri()) return;

  const nativeKey = nativeKeys[key];
  if (!nativeKey) return;
  setAppSetting(nativeKey, value).catch((error) => {
    useToasts().error("Preference couldn't be saved", toUserMessage(error));
  });
}

export function useAppSettings() {
  return { settings: readonly(settings), persistAppSetting };
}

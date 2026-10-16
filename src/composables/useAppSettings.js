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
  historyEnabled: true,
  historyRetentionDays: 30,
  editorFontSize: 13,
  displayTimeZone: "",
  editorFontFamily: "",
  editorTabSize: 2,
  editorWordWrap: false,
  editorKeywordCase: "upper",
  editorVim: false,
  autoLockMinutes: 15,
};
const nativeKeys = {
  themeMode: "theme_mode",
  sidebarWidth: "sidebar_width",
  sidebarOpen: "sidebar_open",
  queryTimeoutSeconds: "query_timeout_seconds",
  maxResultRows: "max_result_rows",
  historyEnabled: "history_enabled",
  historyRetentionDays: "history_retention_days",
  editorFontSize: "editor_font_size",
  displayTimeZone: "display_time_zone",
  editorFontFamily: "editor_font_family",
  editorTabSize: "editor_tab_size",
  editorWordWrap: "editor_word_wrap",
  editorKeywordCase: "editor_keyword_case",
  editorVim: "editor_vim",
  autoLockMinutes: "auto_lock_minutes",
};
const settings = ref({ ...defaults });

/** Settings that act through CSS, like the editor's font size. */
function applyToDocument() {
  document.documentElement.style.setProperty(
    "--editor-font-size",
    `${settings.value.editorFontSize}px`,
  );
  // Validated in Rust to a plain family list, so it is safe in CSS.
  const family = settings.value.editorFontFamily;
  if (family)
    document.documentElement.style.setProperty("--editor-font-family", family);
  else document.documentElement.style.removeProperty("--editor-font-family");
}

export function initializeAppSettings(value) {
  settings.value = { ...defaults, ...value };
  applyToDocument();
}

export function applyLocalAppSetting(key, value) {
  settings.value = { ...settings.value, [key]: value };
  applyToDocument();
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

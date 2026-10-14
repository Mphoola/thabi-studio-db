import { createApp } from "vue";
import App from "./App.vue";
import { installShortcutListener } from "./composables/useCommands.js";
import { initializeAppSettings } from "./composables/useAppSettings.js";
import { initLayout } from "./composables/useLayout.js";
import { initTheme } from "./composables/useTheme.js";
import { useToasts } from "./composables/useToasts.js";
import {
  getAppSettings,
  importLegacyPreferences,
} from "./lib/tauri/commands.js";
import { isTauri } from "./lib/tauri/ipc.js";
import { toUserMessage } from "./lib/errors.js";
import "./styles/main.css";

function readLegacyPreferences() {
  try {
    const themeMode = localStorage.getItem("thabi.theme");
    const layout = JSON.parse(localStorage.getItem("thabi.layout") ?? "{}");
    return {
      themeMode: ["system", "light", "dark"].includes(themeMode)
        ? themeMode
        : null,
      sidebarWidth:
        Number.isInteger(layout.sidebarWidth) &&
        layout.sidebarWidth >= 200 &&
        layout.sidebarWidth <= 360
          ? layout.sidebarWidth
          : null,
      sidebarOpen:
        typeof layout.sidebarOpen === "boolean" ? layout.sidebarOpen : null,
    };
  } catch {
    return { themeMode: null, sidebarWidth: null, sidebarOpen: null };
  }
}

let settings = null;
let settingsError = null;
let migrationError = null;
if (isTauri()) {
  try {
    settings = await importLegacyPreferences(readLegacyPreferences());
  } catch (error) {
    migrationError = error;
    try {
      settings = await getAppSettings();
    } catch (settingsLoadError) {
      settingsError = settingsLoadError;
    }
  }
  if (settings) {
    try {
      localStorage.removeItem("thabi.theme");
      localStorage.removeItem("thabi.layout");
    } catch {
      // SQLite now owns these preferences; stale browser keys are harmless.
    }
  }
}
initializeAppSettings(settings ?? undefined);
initTheme(settings?.themeMode);
initLayout(settings);

const app = createApp(App);
const toasts = useToasts();

// Last-resort handlers. Errors inside the component tree are caught by
// ErrorBoundary first; these cover event handlers and stray promises.
app.config.errorHandler = (error, _instance, info) => {
  console.error(`[vue] ${info}`, error);
  toasts.error("Something went wrong", toUserMessage(error));
};

window.addEventListener("unhandledrejection", (event) => {
  console.error("[unhandledrejection]", event.reason);
  toasts.error("Something went wrong", toUserMessage(event.reason));
});

installShortcutListener();
app.mount("#app");
if (settingsError) {
  toasts.error("Preferences couldn't be loaded", toUserMessage(settingsError));
} else if (migrationError) {
  toasts.error(
    "Previous preferences couldn't be imported",
    toUserMessage(migrationError),
  );
}

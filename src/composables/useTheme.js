import { readonly, ref } from "vue";

/** @typedef {"system" | "light" | "dark"} ThemeMode */

// Interim storage: moves to the SQLite app_settings table in Phase 3.
const STORAGE_KEY = "thabi.theme";
const media = window.matchMedia("(prefers-color-scheme: dark)");
const mode = ref(/** @type {ThemeMode} */ ("system"));
const resolved = ref(/** @type {"light" | "dark"} */ ("light"));

function apply() {
  resolved.value =
    mode.value === "system" ? (media.matches ? "dark" : "light") : mode.value;
  document.documentElement.dataset.theme = resolved.value;
}

/** Call once before mounting. Follows the macOS appearance by default. */
export function initTheme() {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "light" || saved === "dark") mode.value = saved;
  } catch {
    // Storage unavailable: stay on system.
  }
  media.addEventListener("change", apply);
  apply();
}

export function useTheme() {
  /** @param {ThemeMode} next */
  function setMode(next) {
    mode.value = next;
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // Not fatal; the choice lasts for this session.
    }
    apply();
  }
  return { mode: readonly(mode), resolved: readonly(resolved), setMode };
}

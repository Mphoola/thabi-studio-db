import { readonly, ref } from "vue";

/** @typedef {"system" | "light" | "dark"} ThemeMode */

const media = window.matchMedia("(prefers-color-scheme: dark)");
const mode = ref(/** @type {ThemeMode} */ ("system"));
const resolved = ref(/** @type {"light" | "dark"} */ ("light"));

function apply() {
  resolved.value =
    mode.value === "system" ? (media.matches ? "dark" : "light") : mode.value;
  document.documentElement.dataset.theme = resolved.value;
}

/**
 * Call once before mounting. Follows the macOS appearance by default.
 * Persisting the user's choice arrives with app settings (Phase 9).
 */
export function initTheme() {
  media.addEventListener("change", apply);
  apply();
}

export function useTheme() {
  /** @param {ThemeMode} next */
  function setMode(next) {
    mode.value = next;
    apply();
  }
  return { mode: readonly(mode), resolved: readonly(resolved), setMode };
}

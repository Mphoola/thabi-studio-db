import { ref } from "vue";
import { getAppInfo } from "../lib/tauri/commands.js";

const info = ref(
  /** @type {{ name: string, version: string } | null} */ (null),
);
const error = ref(/** @type {string | null} */ (null));
let loading = null;

/** App name and version, read once from the Rust side. */
export function useAppInfo() {
  if (!loading) {
    loading = getAppInfo()
      .then((value) => (info.value = value))
      .catch((err) => (error.value = err?.message ?? String(err)));
  }
  return { info, error };
}

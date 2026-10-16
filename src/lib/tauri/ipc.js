import { invoke } from "@tauri-apps/api/core";

/**
 * True when running inside the Tauri webview (false in a plain browser,
 * e.g. when opening the Vite dev server directly).
 */
export function isTauri() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Single entry point for calling Rust commands. Every command goes through
 * here so errors arrive in one predictable shape: { kind, message }.
 *
 * @param {string} command
 * @param {Record<string, unknown>} [args]
 */
export async function call(command, args) {
  if (!isTauri()) {
    throw {
      kind: "unavailable",
      message:
        "The native backend is only available inside the Thabi Studio app.",
    };
  }
  try {
    return await invoke(command, args);
  } catch (error) {
    if (error && typeof error === "object" && "kind" in error) throw error;
    throw { kind: "ipc", message: String(error) };
  }
}

/** The webview window's label: "main", or "workspace-N" for New Window. */
export async function windowLabel() {
  if (!isTauri()) return "main";
  try {
    const { getCurrentWebviewWindow } =
      await import("@tauri-apps/api/webviewWindow");
    return getCurrentWebviewWindow().label;
  } catch {
    // No window metadata (the browser preview's mock backend): the main one.
    return "main";
  }
}

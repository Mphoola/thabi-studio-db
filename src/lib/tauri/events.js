import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { isTauri } from "./ipc.js";

/**
 * Rust closed a connection on its own: "idle" after an hour unused, or
 * "lost" when a health check failed. Returns an unlisten function.
 *
 * @param {(event: { profileId: string, reason: "idle" | "lost" }) => void} handler
 */
export async function onConnectionClosed(handler) {
  if (!isTauri()) return () => {};
  return listen("connection-closed", (event) => handler(event.payload));
}

/**
 * Items chosen in the native menu bar; the payload is a command id
 * (see src-tauri/src/menu.rs). Returns an unlisten function.
 *
 * @param {(commandId: string) => void} handler
 */
export async function onMenuCommand(handler) {
  if (!isTauri()) return () => {};
  // This window only: Rust sends menu commands to the focused window.
  try {
    return await getCurrentWebviewWindow().listen("menu-command", (event) =>
      handler(event.payload),
    );
  } catch {
    // The browser preview has no native window or menu.
    return () => {};
  }
}

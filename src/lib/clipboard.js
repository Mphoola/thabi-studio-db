import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { isTauri } from "./tauri/ipc.js";

/**
 * Copies text. Uses the native clipboard in the app, because the webview's
 * clipboard API refuses writes that happen after an await.
 */
export async function copyText(text) {
  if (isTauri()) return writeText(text);
  return navigator.clipboard.writeText(text);
}

import { toUserDetail } from "../lib/errors.js";
import {
  exportConnectionProfiles,
  importConnectionProfiles,
} from "../lib/tauri/commands.js";
import {
  pickConnectionsExport,
  pickConnectionsImport,
} from "../lib/tauri/files.js";
import { useConnectionProfiles } from "./useConnectionProfiles.js";
import { useToasts } from "./useToasts.js";

const plural = (n, word) =>
  `${n.toLocaleString()} ${n === 1 ? word : `${word}s`}`;

/** Export and import connections as a JSON file, never with secrets. */
export function useConnectionTransfer() {
  const store = useConnectionProfiles();
  const toasts = useToasts();

  async function exportAll() {
    const path = await pickConnectionsExport();
    if (!path) return;
    try {
      const count = await exportConnectionProfiles(path);
      toasts.success(
        `Exported ${plural(count, "connection")}`,
        "Passwords and passphrases aren’t included; they’re asked for after importing.",
      );
    } catch (error) {
      toasts.error("Couldn’t export connections", toUserDetail(error));
    }
  }

  async function importFile() {
    const path = await pickConnectionsImport();
    if (!path) return;
    try {
      const { imported, skipped } = await importConnectionProfiles(path);
      await store.refresh();
      const detail = skipped.length
        ? skipped.map((s) => `“${s.name}”: ${s.reason}`).join(" ")
        : "Enter each connection’s password the first time you connect.";
      if (imported)
        toasts.success(`Imported ${plural(imported, "connection")}`, detail);
      else toasts.error("No connections were imported", detail);
    } catch (error) {
      toasts.error("Couldn’t import connections", toUserDetail(error));
    }
  }

  return { exportAll, importFile };
}

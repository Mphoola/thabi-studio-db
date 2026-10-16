import { confirm } from "./useConfirm.js";
import { useConnectionProfiles } from "./useConnectionProfiles.js";
import { useHostKeyTrust } from "./useHostKeyTrust.js";
import { useTabs } from "./useTabs.js";
import { useToasts } from "./useToasts.js";
import { toUserDetail } from "../lib/errors.js";
import { pickSqlFile } from "../lib/tauri/files.js";
import {
  readSqlFile,
  runSqlFile,
  sqlFileProgress,
} from "../lib/tauri/commands.js";

const fileName = (path) => path.split(/[\\/]/).pop();

/** Open a .sql file in a tab, or run one statement by statement. */
export function useSqlFiles() {
  const workspace = useTabs();
  const store = useConnectionProfiles();
  const hostKeys = useHostKeyTrust();
  const toasts = useToasts();

  async function openFile() {
    const path = await pickSqlFile();
    if (!path) return;
    try {
      const sql = await readSqlFile(path);
      workspace.newQuery({ sql, title: fileName(path) });
    } catch (error) {
      toasts.error("Couldn’t open the file", toUserDetail(error));
    }
  }

  /**
   * Runs a file on the active query tab's connection and database. The
   * file isn't read into the editor, so it can be large; it always asks
   * first, and a read-only connection refuses.
   */
  async function runFile() {
    const tab =
      workspace.active.value?.type === "query"
        ? workspace.active.value
        : workspace.newQuery();
    const profile = store.profiles.value.find((p) => p.id === tab.profileId);
    if (!profile) {
      toasts.error(
        "Choose a connection first",
        "Run SQL File runs on a tab’s connection.",
      );
      return;
    }
    if (profile.safeMode === "read-only") {
      toasts.error(
        `“${profile.name}” is read-only`,
        "A SQL file can change data, so it isn’t run on a read-only connection.",
      );
      return;
    }
    const path = await pickSqlFile("Run SQL File");
    if (!path) return;
    const where = tab.database ? ` (${tab.database})` : "";
    const ok = await confirm({
      title: `Run “${fileName(path)}” on “${profile.name}”${where}?`,
      message:
        "Every statement in the file runs in order and stops at the first error. Statements that already ran stay done.",
      confirmLabel: "Run File",
      destructive: profile.environment === "production",
      requireText: profile.environment === "production" ? tab.database : null,
    });
    if (!ok) return;

    if (!store.isConnected(profile.id))
      await hostKeys.run(() => store.connect(profile.id));
    tab.running = true;
    tab.runs = [
      {
        kind: "script",
        sql: `-- ${path}`,
        status: "running",
        file: fileName(path),
      },
    ];
    tab.activeRun = 0;
    const entry = tab.runs[0];
    const poll = window.setInterval(async () => {
      const progress = await sqlFileProgress(profile.id, tab.id).catch(
        () => null,
      );
      if (progress) entry.scriptProgress = progress;
    }, 500);
    try {
      entry.script = await runSqlFile(profile.id, tab.id, {
        database: tab.database,
        path,
      });
      entry.status = entry.script.errors.length
        ? "failed"
        : entry.script.cancelled
          ? "cancelled"
          : "completed";
      entry.error = entry.script.errors[0]
        ? {
            message: `Line ${entry.script.errors[0].line}: ${entry.script.errors[0].message}`,
            detail: entry.script.errors[0].detail,
          }
        : null;
    } catch (error) {
      entry.status = "failed";
      entry.error = error;
    } finally {
      window.clearInterval(poll);
      tab.running = false;
    }
  }

  return { openFile, runFile };
}

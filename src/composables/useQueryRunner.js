import { cancelStatement, runStatement } from "../lib/tauri/commands.js";
import { confirmationFor } from "../lib/sqlSafety.js";
import { useTarget } from "../lib/sqlStatements.js";
import { confirm } from "./useConfirm.js";
import { useAppSettings } from "./useAppSettings.js";
import { useConnectionProfiles } from "./useConnectionProfiles.js";
import { useHostKeyTrust } from "./useHostKeyTrust.js";
import { useLibrary } from "./useLibrary.js";

/**
 * Runs statements for a query tab, one after another, on the tab's own
 * session. Stops at the first error. Never retries: if the connection drops
 * the outcome of a write is unknown, so the user decides what to do.
 *
 * Each run is recorded on the tab as
 * { sql, status: "running" | "completed" | "failed" | "cancelled" | "timed_out",
 *   result?, error?, elapsedMs }.
 */
export function useQueryRunner() {
  const store = useConnectionProfiles();
  const hostKeys = useHostKeyTrust();
  const { settings } = useAppSettings();

  async function ensureConnected(profileId) {
    if (store.isConnected(profileId)) return;
    await hostKeys.run(() => store.connect(profileId));
  }

  /**
   * @param {object} tab  a query tab from useTabs
   * @param {string[]} statements
   */
  async function run(tab, statements) {
    if (tab.running || !statements.length) return;
    const profile = store.profiles.value.find((p) => p.id === tab.profileId);
    if (!profile)
      throw {
        kind: "no_connection",
        message: "Choose a connection for this tab first.",
      };

    const question = confirmationFor(statements, {
      environment: profile.environment,
      database: tab.database,
      connection: profile.name,
    });
    if (question && !(await confirm(question))) return;

    await ensureConnected(profile.id);

    tab.running = true;
    tab.startedAt = Date.now();
    tab.runs = [];
    tab.activeRun = 0;
    try {
      for (const sql of statements) {
        tab.runs.push({
          sql,
          status: "running",
          result: null,
          error: null,
          elapsedMs: 0,
        });
        // Update the reactive copy so the results panel follows along.
        const entry = tab.runs[tab.runs.length - 1];
        tab.activeRun = tab.runs.length - 1;
        const started = performance.now();
        try {
          entry.result = await runStatement(profile.id, tab.id, {
            database: tab.database,
            sql,
            timeoutSeconds: settings.value.queryTimeoutSeconds || null,
            maxRows: settings.value.maxResultRows || null,
          });
          entry.status = "completed";
          // `USE db` moved the tab's session; keep the picker in step.
          const used = profile.driver === "postgres" ? null : useTarget(sql);
          if (used) tab.database = used;
          entry.elapsedMs = entry.result.elapsedMs;
        } catch (error) {
          entry.error = error;
          entry.elapsedMs = Math.round(performance.now() - started);
          entry.status =
            error?.kind === "query_cancelled"
              ? "cancelled"
              : error?.kind === "query_timed_out"
                ? "timed_out"
                : "failed";
          if (error?.kind === "connection_lost") store.forgetStatus(profile.id);
          break;
        }
      }
    } finally {
      tab.running = false;
      tab.startedAt = null;
      useLibrary()
        .refreshHistory()
        .catch(() => {});
    }
  }

  async function cancel(tab) {
    if (!tab?.running || !tab.profileId) return false;
    return cancelStatement(tab.profileId, tab.id);
  }

  return { run, cancel };
}

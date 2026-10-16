import {
  cancelStatement,
  queryProgress,
  runStatement,
} from "../lib/tauri/commands.js";
import { limitsFor } from "../lib/limits.js";
import { confirmationFor, writeBlockFor } from "../lib/sqlSafety.js";
import { useTarget } from "../lib/sqlStatements.js";
import {
  boundValues,
  findParameters,
  parameterLabels,
} from "../lib/queryParameters.js";
import { askParameters } from "./useParameterPrompt.js";
import { confirm } from "./useConfirm.js";
import { useAppSettings } from "./useAppSettings.js";
import { useConnectionProfiles } from "./useConnectionProfiles.js";
import { useHostKeyTrust } from "./useHostKeyTrust.js";
import { useLibrary } from "./useLibrary.js";
import { useToasts } from "./useToasts.js";

/** How often a running statement's progress is read. */
const PROGRESS_MS = 500;

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
  const toasts = useToasts();
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

    // A read-only connection refuses these before anything is sent.
    const blocked = writeBlockFor(statements, profile.safeMode);
    if (blocked) {
      toasts.error(`“${profile.name}” is read-only`, blocked);
      return;
    }

    const limits = limitsFor(profile, settings.value);
    const question = confirmationFor(statements, {
      safeMode: profile.safeMode,
      environment: profile.environment,
      database: tab.database,
      connection: profile.name,
    });
    if (question && !(await confirm(question))) return;

    // Placeholders are asked for once, across every statement, and bound.
    const prepared = await prepareParameters(tab, statements, profile.driver);
    if (!prepared) return;

    await ensureConnected(profile.id);

    tab.running = true;
    tab.startedAt = Date.now();
    tab.runs = [];
    tab.activeRun = 0;
    try {
      for (const [index, sql] of statements.entries()) {
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
        // While it runs, show how many rows have arrived and the first ones.
        const poll = window.setInterval(async () => {
          try {
            const progress = await queryProgress(profile.id, tab.id);
            if (progress && entry.status === "running")
              entry.progress = progress;
          } catch {
            // Progress is a nicety; the run itself reports any failure.
          }
        }, PROGRESS_MS);
        try {
          entry.result = await runStatement(profile.id, tab.id, {
            database: tab.database,
            sql: prepared[index].sql,
            bound: prepared[index].bound,
            timeoutSeconds: limits.timeoutSeconds,
            maxRows: limits.maxRows,
          });
          entry.status = "completed";
          // A batch's further row sets each get a result tab of their own.
          for (const [part, more] of (entry.result.more ?? []).entries())
            tab.runs.push({
              sql,
              status: "completed",
              result: more,
              error: null,
              elapsedMs: more.elapsedMs,
              part: part + 2,
            });
          // `USE db` moved the tab's session; keep the picker in step.
          const used = profile.driver === "postgres" ? null : useTarget(sql);
          if (used) tab.database = used;
          entry.elapsedMs = entry.result.elapsedMs;
        } catch (error) {
          entry.error = error;
          // A cancel or timeout keeps the rows that had already arrived.
          if (error?.data?.partial) entry.result = error.data.partial;
          entry.elapsedMs = Math.round(performance.now() - started);
          entry.status =
            error?.kind === "query_cancelled"
              ? "cancelled"
              : error?.kind === "query_timed_out"
                ? "timed_out"
                : "failed";
          if (error?.kind === "connection_lost") store.forgetStatus(profile.id);
          break;
        } finally {
          window.clearInterval(poll);
          entry.progress = null;
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

  /**
   * Each statement as it will be sent: rewritten to the engine's placeholders
   * with its values, or as typed. null when the user cancels the form.
   */
  async function prepareParameters(tab, statements, engine) {
    const found = statements.map((sql) => findParameters(sql, engine));
    const labels = [];
    for (const f of found)
      for (const label of parameterLabels(f?.names ?? []))
        if (!labels.includes(label)) labels.push(label);
    if (!labels.length) return statements.map((sql) => ({ sql, bound: null }));
    const remembered = tab.parameterValues ?? {};
    const answers = await askParameters({
      labels,
      defaults: labels.map((l) => remembered[l]),
    });
    if (!answers) return null;
    tab.parameterValues = {
      ...remembered,
      ...Object.fromEntries(labels.map((l, i) => [l, answers[i]])),
    };
    return statements.map((sql, i) => {
      const f = found[i];
      if (!f) return { sql, bound: null };
      const mine = parameterLabels(f.names).map(
        (label) => answers[labels.indexOf(label)],
      );
      return { sql: f.sql, bound: boundValues(f, mine) };
    });
  }

  async function cancel(tab) {
    if (!tab?.running || !tab.profileId) return false;
    return cancelStatement(tab.profileId, tab.id);
  }

  return { run, cancel };
}

import { watch } from "vue";
import { loadWorkspaceTabs, saveWorkspaceTabs } from "../lib/tauri/commands.js";
import { isTauri } from "../lib/tauri/ipc.js";
import { useTabs } from "./useTabs.js";

const SAVE_DELAY_MS = 800;

/**
 * Keeps open query tabs (title, SQL, connection, database) in the app's
 * database so unsaved work survives quitting or a crash. Call once, after
 * saved connections have loaded.
 */
export async function startTabPersistence() {
  if (!isTauri()) return;
  const workspace = useTabs();

  try {
    const saved = await loadWorkspaceTabs();
    for (const tab of saved) {
      workspace.newQuery({
        id: tab.tabId,
        title: tab.title,
        sql: tab.queryText,
        profileId: tab.profileId,
        database: tab.databaseName,
        savedQueryId: tab.savedQueryId,
        // A restored tab is unsaved unless it matches its saved query; we
        // don't know that here, so only saved-query tabs start clean.
        savedSql: tab.savedQueryId ? tab.queryText : "",
        activate: false,
      });
    }
  } catch (error) {
    console.error("[tabs] couldn't restore open tabs", error);
  }

  let timer = 0;
  const snapshot = () =>
    workspace.tabs.value
      .filter((tab) => tab.type === "query")
      .map((tab) => ({
        tabId: tab.id,
        title: tab.title,
        queryText: tab.sql,
        profileId: tab.profileId,
        databaseName: tab.database,
        savedQueryId: tab.savedQueryId ?? null,
      }));
  watch(
    () => JSON.stringify(snapshot()),
    () => {
      clearTimeout(timer);
      timer = window.setTimeout(() => {
        saveWorkspaceTabs(snapshot()).catch((error) =>
          console.error("[tabs] couldn't save open tabs", error),
        );
      }, SAVE_DELAY_MS);
    },
  );
}

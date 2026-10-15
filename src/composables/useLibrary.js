import { computed, ref } from "vue";
import {
  clearHistory,
  deleteHistoryEntry,
  deleteSavedQuery,
  listHistory,
  listSavedQueries,
  renameSavedFolder,
  saveQuery,
} from "../lib/tauri/commands.js";
import { isTauri } from "../lib/tauri/ipc.js";

/** Query history and saved queries, as shown in the sidebar. */
const history = ref([]);
const historySearch = ref("");
const saved = ref([]);
const loaded = ref(false);
/** Folder names in use, sorted. */
const folders = computed(() =>
  [...new Set(saved.value.map((q) => q.folder).filter(Boolean))].sort((a, b) =>
    a.localeCompare(b, undefined, { sensitivity: "base" }),
  ),
);

async function refreshHistory() {
  if (!isTauri()) return;
  history.value = await listHistory({
    search: historySearch.value,
    limit: 200,
  });
}

async function refreshSaved() {
  if (!isTauri()) return;
  saved.value = await listSavedQueries();
}

export function useLibrary() {
  async function load() {
    if (loaded.value) return;
    loaded.value = true;
    await Promise.all([refreshHistory(), refreshSaved()]);
  }

  async function save(input) {
    const result = await saveQuery(input);
    await refreshSaved();
    return result;
  }

  async function removeSaved(id) {
    await deleteSavedQuery(id);
    await refreshSaved();
  }

  async function renameFolder(from, to) {
    await renameSavedFolder(from, to);
    await refreshSaved();
  }

  async function removeHistory(id) {
    await deleteHistoryEntry(id);
    history.value = history.value.filter((entry) => entry.id !== id);
  }

  async function clearAll() {
    const count = await clearHistory();
    history.value = [];
    return count;
  }

  return {
    history,
    historySearch,
    saved,
    folders,
    load,
    refreshHistory,
    refreshSaved,
    save,
    removeSaved,
    renameFolder,
    removeHistory,
    clearAll,
  };
}

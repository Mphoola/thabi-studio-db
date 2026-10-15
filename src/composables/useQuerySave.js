import { ref } from "vue";
import { useDialogs } from "./useDialogs.js";
import { useLibrary } from "./useLibrary.js";
import { useTabs } from "./useTabs.js";
import { useToasts } from "./useToasts.js";
import { toUserMessage } from "../lib/errors.js";

/** The tab or saved query the Save Query dialog is working on. */
const pending = ref(null); // { tab?, query? }

/** Save Query (Mod+S): updates a saved query, or asks for a name first. */
export function useQuerySave() {
  const workspace = useTabs();
  const library = useLibrary();
  const dialogs = useDialogs();
  const toasts = useToasts();

  async function saveActive() {
    const tab = workspace.active.value;
    if (tab?.type !== "query") return;
    if (!tab.savedQueryId) {
      pending.value = { tab };
      dialogs.show("saveQuery");
      return;
    }
    // Keep the notes and folder the query already has.
    const existing = library.saved.value.find((q) => q.id === tab.savedQueryId);
    try {
      await library.save({
        id: tab.savedQueryId,
        name: tab.title,
        queryText: tab.sql,
        profileId: tab.profileId,
        notes: existing?.notes ?? "",
        folder: existing?.folder ?? "",
      });
      tab.savedSql = tab.sql;
    } catch (error) {
      toasts.error(`Couldn’t save “${tab.title}”`, toUserMessage(error));
    }
  }

  /** Opens the dialog to rename a saved query, move it or edit its notes. */
  function editSaved(query) {
    pending.value = { query };
    dialogs.show("saveQuery");
  }

  /** Called by the dialog. Returns field errors, or null when saved. */
  async function submit({ name, notes, folder }) {
    const { tab, query } = pending.value ?? {};
    try {
      const saved = await library.save({
        id: query?.id ?? null,
        name,
        notes,
        folder,
        queryText: query ? query.queryText : tab.sql,
        profileId: query ? query.profileId : tab.profileId,
      });
      if (tab) {
        tab.title = saved.name;
        tab.savedQueryId = saved.id;
        tab.savedSql = tab.sql;
      }
      // Keep any open tab of a renamed query in step.
      for (const open of workspace.tabs.value)
        if (open.savedQueryId === saved.id) open.title = saved.name;
      dialogs.close();
      pending.value = null;
      return null;
    } catch (error) {
      if (error?.kind === "validation") return error.fields ?? {};
      toasts.error("Couldn’t save the query", toUserMessage(error));
      return {};
    }
  }

  return { pending, saveActive, editSaved, submit };
}

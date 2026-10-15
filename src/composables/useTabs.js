import { computed, reactive, ref } from "vue";
import { closeQueryTab } from "../lib/tauri/commands.js";
import { confirm } from "./useConfirm.js";
import { useConnectionProfiles } from "./useConnectionProfiles.js";
import { useLayout } from "./useLayout.js";

/**
 * Workspace tabs: query editors and opened tables/views. Query tabs keep
 * their SQL and results in memory for this session (saving and recovery
 * arrive in Phase 9).
 */
const tabs = ref([]);
/** Run/cancel/focus handles registered by mounted query tabs. */
const handles = new Map();
const activeId = ref(null);
let counter = 0;

function newId() {
  counter += 1;
  return `tab-${Date.now().toString(36)}-${counter}`;
}

export function useTabs() {
  const layout = useLayout();
  const store = useConnectionProfiles();

  const active = computed(
    () => tabs.value.find((tab) => tab.id === activeId.value) ?? null,
  );

  function show(tab) {
    activeId.value = tab.id;
    layout.mainView.value = "workspace";
  }

  /**
   * Opens a new query tab. `savedSql` is what "unsaved" is measured against:
   * the saved query's text, or nothing for a new tab.
   */
  function newQuery({
    sql = "",
    database = null,
    title = null,
    profileId = undefined,
    savedQueryId = null,
    savedSql = null,
    id = null,
    activate = true,
  } = {}) {
    const profile =
      profileId === undefined
        ? store.selectedProfile.value
        : (store.profiles.value.find((p) => p.id === profileId) ?? null);
    const number = tabs.value.filter((t) => t.type === "query").length + 1;
    const tab = reactive({
      id: id ?? newId(),
      type: "query",
      title: title ?? `Query ${number}`,
      profileId: profileId === undefined ? (profile?.id ?? null) : profileId,
      database:
        database ??
        (profile ? store.statusFor(profile.id)?.server?.database : null) ??
        profile?.databaseName ??
        null,
      sql,
      savedSql: savedSql ?? (savedQueryId ? sql : ""),
      savedQueryId,
      running: false,
      startedAt: null,
      runs: [],
      activeRun: 0,
    });
    tabs.value.push(tab);
    if (activate) show(tab);
    return tab;
  }

  /** Opens a saved query, or switches to its tab if it's already open. */
  function openSaved(query) {
    const open = tabs.value.find((t) => t.savedQueryId === query.id);
    if (open) return show(open);
    return newQuery({
      sql: query.queryText,
      title: query.name,
      savedQueryId: query.id,
      profileId: query.profileId ?? undefined,
    });
  }

  function openObject(node, view = "data") {
    const profileId = store.selectedProfile.value?.id ?? null;
    const existing = tabs.value.find(
      (t) =>
        t.type === "object" &&
        t.node.key === node.key &&
        t.profileId === profileId,
    );
    if (existing) {
      existing.view = view;
      return show(existing);
    }
    const tab = reactive({
      id: newId(),
      type: "object",
      title: node.name,
      profileId,
      node,
      view,
    });
    tabs.value.push(tab);
    show(tab);
  }

  const hasStagedEdits = (tab) =>
    Boolean(
      tab.edits &&
      (Object.keys(tab.edits.updates).length ||
        Object.keys(tab.edits.deletes).length ||
        tab.edits.inserts.length),
    );
  const isDirty = (tab) =>
    tab.type === "query"
      ? tab.sql.trim() !== tab.savedSql.trim()
      : hasStagedEdits(tab);

  async function close(id = activeId.value) {
    const index = tabs.value.findIndex((tab) => tab.id === id);
    if (index < 0) return false;
    const tab = tabs.value[index];
    if (tab.running) {
      const ok = await confirm({
        title: `Close “${tab.title}” while it runs?`,
        message: "The running query will be stopped.",
        confirmLabel: "Stop and Close",
        destructive: true,
      });
      if (!ok) return false;
    } else if (isDirty(tab)) {
      const ok = await confirm({
        title: `Close “${tab.title}”?`,
        message:
          tab.type === "query"
            ? "Its changes aren't saved and will be lost. Save it first with Save Query if you want to keep it."
            : "Its staged row changes haven't been applied and will be lost.",
        confirmLabel: "Close",
        destructive: true,
      });
      if (!ok) return false;
    }
    tabs.value.splice(index, 1);
    // Query and object tabs each have their own database session.
    if (tab.profileId) closeQueryTab(tab.profileId, tab.id).catch(() => {});
    if (activeId.value === id) {
      const next = tabs.value[index] ?? tabs.value[index - 1] ?? null;
      activeId.value = next?.id ?? null;
    }
    return true;
  }

  /** The active query tab's handle ({ run, cancel, focus }), if any. */
  function activeHandle() {
    return active.value?.type === "query" ? handles.get(active.value.id) : null;
  }

  return {
    tabs,
    activeHandle,
    registerHandle: (id, handle) => handles.set(id, handle),
    unregisterHandle: (id) => handles.delete(id),
    activeId,
    active,
    show,
    newQuery,
    openSaved,
    openObject,
    close,
    isDirty,
  };
}

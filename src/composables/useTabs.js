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
/** Two query tabs shown side by side ([left, right]), or null. */
const split = ref(null);
let counter = 0;

function newId() {
  counter += 1;
  // Unique across windows too: every tab id names its own database session.
  const random = Math.random().toString(36).slice(2, 8);
  return `tab-${Date.now().toString(36)}-${counter}-${random}`;
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

  /**
   * Opens a table or view. `filters` (filter bar rows) opens it already
   * filtered, as a foreign key jump does; an open tab is re-filtered.
   */
  function openObject(node, view = "data", { filters = null } = {}) {
    const profileId = store.selectedProfile.value?.id ?? null;
    const existing = tabs.value.find(
      (t) =>
        t.type === "object" &&
        t.node.key === node.key &&
        t.profileId === profileId,
    );
    if (existing) {
      existing.view = view;
      if (filters) {
        existing.filters = filters;
        existing.showFilters = true;
        existing.reloadToken = (existing.reloadToken ?? 0) + 1;
      }
      return show(existing);
    }
    const tab = reactive({
      id: newId(),
      type: "object",
      title: node.name,
      profileId,
      node,
      view,
      ...(filters ? { filters, showFilters: true } : {}),
    });
    tabs.value.push(tab);
    show(tab);
  }

  /**
   * Opens the ER diagram for a schema. Read-only: nothing here can change
   * the database, which is why it is a tab of its own rather than a view of
   * somebody else's tab.
   */
  function openDiagram({ profileId, database, schema, title }) {
    const tab = reactive({
      id: newId(),
      type: "diagram",
      title: title ?? `${database} diagram`,
      profileId,
      database,
      schema: schema ?? null,
    });
    tabs.value.push(tab);
    show(tab);
    return tab;
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
    if (split.value?.includes(id)) split.value = null;
    // Query and object tabs each have their own database session.
    if (tab.profileId) closeQueryTab(tab.profileId, tab.id).catch(() => {});
    if (activeId.value === id) {
      const next = tabs.value[index] ?? tabs.value[index - 1] ?? null;
      activeId.value = next?.id ?? null;
    }
    return true;
  }

  /** Both split panes are visible when the active tab is one of them. */
  const splitPanes = computed(() =>
    split.value?.includes(activeId.value) ? split.value : null,
  );

  /**
   * Split Editor: shows a second query tab beside the active one, on the
   * same connection and database; again, goes back to one pane.
   */
  function toggleSplit() {
    if (splitPanes.value) {
      split.value = null;
      return;
    }
    const current = active.value;
    if (current?.type !== "query") return;
    const other = newQuery({
      profileId: current.profileId,
      database: current.database,
      activate: false,
    });
    split.value = [current.id, other.id];
    activeId.value = other.id;
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
    splitPanes,
    toggleSplit,
    show,
    newQuery,
    openSaved,
    openObject,
    openDiagram,
    close,
    isDirty,
  };
}

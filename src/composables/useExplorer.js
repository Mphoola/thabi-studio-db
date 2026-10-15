import { computed, reactive, ref } from "vue";
import {
  databaseNode,
  objectGroups,
  schemaNode,
  visibleRows,
} from "../lib/explorerTree.js";
import {
  listDatabases,
  listObjects,
  listSchemas,
} from "../lib/tauri/commands.js";
import { useConnectionProfiles } from "./useConnectionProfiles.js";
import { useLayout } from "./useLayout.js";

/**
 * Explorer state for the selected, connected profile. Children load when a
 * node is first expanded and stay cached until refreshed. Everything resets
 * when the connection changes.
 */
const loaded = reactive(new Map()); // key -> { status, children?, error? }
const expanded = ref(new Set());
const selectedKey = ref(null);
const openObject = ref(null); // { node, tab }
let forConnection = null;

const ROOT = "root";

function reset() {
  loaded.clear();
  expanded.value = new Set();
  selectedKey.value = null;
  openObject.value = null;
}

export function useExplorer() {
  const store = useConnectionProfiles();
  const layout = useLayout();

  const profile = store.selectedProfile;
  const status = computed(() =>
    profile.value ? store.statusFor(profile.value.id) : null,
  );
  const connected = computed(() => status.value?.state === "connected");

  // Start fresh whenever a different connection (or reconnection) appears.
  function syncConnection() {
    const id = connected.value ? status.value.connectionId : null;
    if (id !== forConnection) {
      forConnection = id;
      reset();
    }
  }

  async function fetchChildren(node) {
    const id = profile.value.id;
    if (!node) {
      const current = status.value.server?.database;
      const names = await listDatabases(id);
      // Put the database this connection opened first.
      names.sort((a, b) => (a === current ? -1 : b === current ? 1 : 0));
      return names.map(databaseNode);
    }
    if (node.type === "database") {
      const schemas = await listSchemas(id, node.database);
      if (schemas === null)
        return objectGroups(node, await listObjects(id, node.database));
      return schemas.map((schema) => schemaNode(node.database, schema));
    }
    if (node.type === "schema")
      return objectGroups(
        node,
        await listObjects(id, node.database, node.schema),
      );
    return [];
  }

  async function load(node = null) {
    syncConnection();
    if (!connected.value) return;
    const key = node?.key ?? ROOT;
    const connection = forConnection;
    loaded.set(key, { ...(loaded.get(key) ?? {}), status: "loading" });
    try {
      const children = await fetchChildren(node);
      if (connection !== forConnection) return;
      loaded.set(key, { status: "loaded", children });
    } catch (error) {
      if (connection !== forConnection) return;
      loaded.set(key, { status: "error", error });
      if (error?.kind === "connection_lost")
        store.forgetStatus(profile.value.id);
    }
  }

  function stateOf(key) {
    return loaded.get(key) ?? { status: "idle" };
  }

  async function toggle(node) {
    const next = new Set(expanded.value);
    if (next.has(node.key)) next.delete(node.key);
    else {
      next.add(node.key);
      if (!node.children && stateOf(node.key).status === "idle") load(node);
    }
    expanded.value = next;
  }

  function expand(node, open) {
    if (expanded.value.has(node.key) !== open) toggle(node);
  }

  const roots = computed(() => stateOf(ROOT).children ?? []);
  const rows = computed(() =>
    visibleRows(roots.value, expanded.value, (key) => stateOf(key).children),
  );

  function open(node, tab = "data") {
    selectedKey.value = node.key;
    openObject.value = { node, tab };
    layout.mainView.value = "object";
  }

  async function refresh(node = null) {
    if (!node) {
      reset();
      forConnection = connected.value ? status.value.connectionId : null;
      return load();
    }
    if (node.type === "group" || node.type === "object") {
      const parentKey = node.schema
        ? `schema:${node.database}\u0000${node.schema}`
        : `db:${node.database}`;
      const parent = [...rows.value].find((r) => r.node.key === parentKey);
      if (parent) return load(parent.node);
    }
    return load(node);
  }

  return {
    profile,
    status,
    connected,
    rows,
    expanded,
    selectedKey,
    openObject,
    stateOf,
    rootState: computed(() => stateOf(ROOT)),
    load,
    toggle,
    expand,
    open,
    refresh,
    syncConnection,
  };
}

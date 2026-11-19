import { forgetCompletionSchema } from "./useCompletionSchema.js";
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
  listRoutines,
  listSchemas,
  routineDefinition,
} from "../lib/tauri/commands.js";
import { toUserDetail } from "../lib/errors.js";
import { useToasts } from "./useToasts.js";
import { useConnectionProfiles } from "./useConnectionProfiles.js";
import { useTabs } from "./useTabs.js";

/**
 * Explorer state for the selected, connected profile. Children load when a
 * node is first expanded and stay cached until refreshed. Everything resets
 * when the connection changes.
 */
const loaded = reactive(new Map()); // key -> { status, children?, error? }
const expanded = ref(new Set());
const selectedKey = ref(null);
let forConnection = null;

const ROOT = "root";

function reset() {
  loaded.clear();
  expanded.value = new Set();
  selectedKey.value = null;
}

export function useExplorer() {
  const store = useConnectionProfiles();
  const workspace = useTabs();

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

  /** Tables and views, and the routines beside them. A login that can't
   *  read routines still sees its tables: that list is not lost with it. */
  async function groupsOf(id, node) {
    const [objects, routines] = await Promise.all([
      listObjects(id, node.database, node.schema ?? undefined),
      listRoutines(id, node.database, node.schema ?? undefined).catch(() => []),
    ]);
    return objectGroups(node, objects, routines);
  }

  async function fetchChildren(node) {
    const id = profile.value.id;
    if (!node) {
      const current = status.value.server?.database;
      // A connection saved with a database shows only that database.
      const scoped = profile.value.databaseName
        ? current || profile.value.databaseName
        : null;
      if (scoped) return [databaseNode(scoped)];
      const names = await listDatabases(id);
      // Put the database this connection opened first.
      names.sort((a, b) => (a === current ? -1 : b === current ? 1 : 0));
      return names.map(databaseNode);
    }
    if (node.type === "database") {
      const schemas = await listSchemas(id, node.database);
      if (schemas === null) return groupsOf(id, node);
      return schemas.map((schema) => schemaNode(node.database, schema));
    }
    if (node.type === "schema") return groupsOf(id, node);
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

  function open(node, view = "data") {
    selectedKey.value = node.key;
    if (node.type === "routine") return openDefinition(node);
    workspace.openObject(node, view);
  }

  /**
   * A routine opens as its definition in a query tab: read it, change it to
   * CREATE OR REPLACE or ALTER, and run it like any statement, with the
   * connection's safe mode asking first.
   */
  async function openDefinition(node) {
    try {
      const sql = await routineDefinition(
        profile.value.id,
        node.database,
        node.kind,
        node.identity,
      );
      workspace.newQuery({
        sql,
        database: node.database,
        title: `${node.kind[0].toUpperCase()}${node.kind.slice(1)} ${node.name}`,
      });
    } catch (error) {
      useToasts().error(`Couldn’t read “${node.name}”`, toUserDetail(error));
    }
  }

  async function refresh(node = null) {
    // New tables or columns should show up in autocomplete too.
    if (profile.value) forgetCompletionSchema(profile.value.id);
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

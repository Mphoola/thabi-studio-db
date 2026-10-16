import { ref, watch } from "vue";
import { useTabs } from "../../composables/useTabs.js";
import { objectNode } from "../../lib/explorerTree.js";
import { newFilter } from "../../lib/rowFilters.js";
import { listReferencing } from "../../lib/tauri/commands.js";

/**
 * Foreign key navigation for a table tab: from a foreign key cell to the row
 * it points at, and from a referenced cell to the rows that point at it.
 * Each jump opens (or reuses) the other table's tab, already filtered.
 */
export function useTableLinks({ node, profileId, columnInfo }) {
  const workspace = useTabs();
  const referencing = ref([]);

  watch(
    node,
    async (target) => {
      referencing.value = [];
      if (target?.kind !== "table" || !profileId.value) return;
      try {
        const list = await listReferencing(profileId.value, target);
        if (node.value === target) referencing.value = list;
      } catch {
        // Navigation is a convenience: without the list, the menu just has
        // fewer entries.
      }
    },
    { immediate: true },
  );

  function open(schema, table, column, value) {
    const database = node.value.database;
    // MySQL names the database where the others name a schema.
    const isMysql = !node.value.schema;
    const target = isMysql
      ? objectNode(schema || database, null, table)
      : objectNode(database, schema, table);
    workspace.openObject(target, "data", {
      filters: [{ ...newFilter(column), value }],
    });
  }

  /** Jumps available from one cell: [{ label, run }]. */
  function cellLinks(columnName, cell) {
    if (cell?.kind !== "value") return [];
    const value = cell.value;
    const links = [];
    const ref = columnInfo.value.get(columnName)?.references;
    if (ref)
      links.push({
        label: `Open ${ref.table} Row`,
        run: () => open(ref.schema, ref.table, ref.column, value),
      });
    for (const child of referencing.value)
      if (child.referencedColumn === columnName)
        links.push({
          label: `Rows in ${child.table}.${child.column}`,
          run: () => open(child.schema, child.table, child.column, value),
        });
    return links;
  }

  return { cellLinks };
}

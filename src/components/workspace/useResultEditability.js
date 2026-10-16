import { ref, watch } from "vue";
import {
  resultEditability,
  singleTableSelect,
} from "../../lib/queryEditability.js";
import { describeObject } from "../../lib/tauri/commands.js";

/**
 * Whether the current query result can be edited, and the table it reads
 * (for SQL copy/export formats, via `resultTarget`).
 */
export function useResultEditability({ tab, run, resultTarget }) {
  /**
   * Editing a query's result, when it is safe.
   *
   * The same staged editing a table tab uses, pointed at the table the statement
   * reads. Safe means one plainly-read table whose whole key is in the result and
   * whose rows are not repeated — otherwise a cell would be edited by row number
   * rather than by key, and the write would land somewhere else. See
   * lib/queryEditability.js, which is where that is decided and tested.
   */
  const editNode = ref(null);
  const whyNotEditable = ref("");

  async function checkEditability(entry) {
    editNode.value = null;
    whyNotEditable.value = "";
    resultTarget.value = null;
    if (!entry?.result?.columns.length || !tab.profileId) return;
    const parsed = singleTableSelect(entry.sql);
    if (!parsed) {
      whyNotEditable.value =
        "Read-only: only a plain SELECT from one table can be edited from here.";
      return;
    }
    try {
      const structure = await describeObject(tab.profileId, {
        database: tab.database ?? "",
        schema: parsed.schema || null,
        name: parsed.table,
        kind: "table",
      });
      const primaryKey = structure.columns
        .filter((column) => column.primaryKey)
        .sort((a, b) => a.primaryKey - b.primaryKey)
        .map((column) => column.name);
      resultTarget.value = {
        database: tab.database ?? "",
        schema: parsed.schema ?? "",
        name: parsed.table,
        primaryKey,
      };
      const answer = resultEditability({
        statement: entry.sql,
        columns: entry.result.columns,
        primaryKey,
        rows: entry.result.rows ?? [],
        database: tab.database ?? "",
      });
      if (answer.editable)
        editNode.value = {
          database: tab.database ?? "",
          schema: parsed.schema ?? "",
          name: parsed.table,
          kind: "table",
        };
      else whyNotEditable.value = `Read-only: ${answer.reason}`;
    } catch {
      // The structure could not be read, so the key is unknown: stay read-only
      // and say so rather than offering editing that cannot address a row.
      whyNotEditable.value = `Read-only: ${parsed.table} could not be read.`;
    }
  }

  watch(run, (entry) => checkEditability(entry), { immediate: true });

  return { editNode, whyNotEditable };
}

import { computed } from "vue";
import {
  counts,
  createEdits,
  keyParts,
  revertRow,
  rowKey,
  setCell,
  shownCell,
  toChanges,
  toggleDeletes,
} from "../lib/rowEdits.js";

/**
 * Staged edits for one table tab, as the grid's `editor`. Edits live on the
 * tab so they survive switching tabs. Editing needs a primary key whose
 * columns are all in the result.
 * @param tab the object tab (reactive)
 * @param columns computed result columns
 * @param primaryKey computed primary key column names, in key order
 */
export function useRowEdits(tab, columns, primaryKey) {
  if (!tab.edits) tab.edits = createEdits();

  const pkIndexes = computed(() =>
    primaryKey.value.map((name) =>
      columns.value.findIndex((c) => c.name === name),
    ),
  );
  const enabled = computed(
    () => primaryKey.value.length > 0 && !pkIndexes.value.includes(-1),
  );
  const parts = (cells) =>
    enabled.value ? keyParts(cells, columns.value, pkIndexes.value) : null;
  const keyOf = (cells) => rowKey(parts(cells));

  function isDeleted(cells) {
    const key = keyOf(cells);
    return Boolean(key && tab.edits.deletes[key]);
  }

  function display(cells, col) {
    const key = keyOf(cells);
    return key
      ? shownCell(tab.edits, key, columns.value[col].name, cells[col])
      : cells[col];
  }

  return {
    enabled,
    summary: computed(() => counts(tab.edits)),
    canEdit: (cells, col) =>
      Boolean(parts(cells)) &&
      !isDeleted(cells) &&
      cells[col]?.kind !== "binary",
    isDeleted,
    isChanged(cells, col) {
      const key = keyOf(cells);
      const set = key && tab.edits.updates[key]?.set;
      return Boolean(set && columns.value[col].name in set);
    },
    display,
    /** The value an editor should start with: staged or original. */
    valueOf(cells, col) {
      const cell = display(cells, col);
      return cell?.kind === "value" ? cell.value : null;
    },
    setValue(cells, col, value) {
      const p = parts(cells);
      if (p) setCell(tab.edits, p, columns.value[col].name, cells[col], value);
    },
    /** Toggles deletion for rows; returns how many rows could be marked. */
    toggleDelete(rowsCells) {
      const all = rowsCells.map(parts).filter(Boolean);
      if (all.length) toggleDeletes(tab.edits, all);
      return all.length;
    },
    revert(cells) {
      const p = parts(cells);
      if (p) revertRow(tab.edits, p);
    },
    addRow(values) {
      tab.edits.inserts.push(values);
    },
    discard() {
      tab.edits = createEdits();
    },
    changes: () => toChanges(tab.edits),
  };
}

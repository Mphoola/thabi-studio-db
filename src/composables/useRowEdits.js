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

  /**
   * A source column the editor knows about.
   *
   * The grid can ask about a column the current result does not have — a result
   * whose shape changed while the grid still holds a row from the old one. This
   * is called during render, so it must answer rather than throw: returning
   * undefined makes every operation on that column a no-op, and the grid falls
   * back to the raw cell.
   */
  const name = (col) => columns.value[col]?.name;

  function display(cells, col) {
    const column = name(col);
    const key = column ? keyOf(cells) : null;
    return key ? shownCell(tab.edits, key, column, cells[col]) : cells[col];
  }

  return {
    enabled,
    summary: computed(() => counts(tab.edits)),
    canEdit: (cells, col) =>
      Boolean(name(col)) &&
      Boolean(parts(cells)) &&
      !isDeleted(cells) &&
      cells[col]?.kind !== "binary",
    isDeleted,
    isChanged(cells, col) {
      const column = name(col);
      const key = column ? keyOf(cells) : null;
      const set = key && tab.edits.updates[key]?.set;
      return Boolean(set && column in set);
    },
    display,
    /** The value an editor should start with: staged or original. */
    valueOf(cells, col) {
      const cell = display(cells, col);
      return cell?.kind === "value" ? cell.value : null;
    },
    setValue(cells, col, value) {
      const column = name(col);
      const p = column ? parts(cells) : null;
      if (p) setCell(tab.edits, p, column, cells[col], value);
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

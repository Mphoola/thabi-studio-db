import { computed, ref } from "vue";
import {
  DEFAULT_VALUE,
  counts,
  createEdits,
  keyParts,
  revertRow,
  rowKey,
  setCell,
  shownCell,
  toChanges,
  parsePastedRows,
  toggleDeletes,
} from "../lib/rowEdits.js";

/** Undo steps kept per tab. */
const UNDO_LIMIT = 100;

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
  // Snapshots of the staged edits for undo/redo. Kept on the tab, so they
  // survive switching tabs, but not saved anywhere.
  if (!tab.editHistory) tab.editHistory = { past: [], future: [] };
  const history = tab.editHistory;
  const version = ref(0); // bumps so canUndo/canRedo re-evaluate

  /** Runs a change to the staged edits as one undo step. */
  function step(change) {
    const before = JSON.stringify(tab.edits);
    change();
    if (JSON.stringify(tab.edits) === before) return;
    history.past.push(before);
    if (history.past.length > UNDO_LIMIT) history.past.shift();
    history.future = [];
    version.value += 1;
  }

  function travel(from, to) {
    if (!from.length) return false;
    to.push(JSON.stringify(tab.edits));
    tab.edits = JSON.parse(from.pop());
    version.value += 1;
    return true;
  }

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
      if (p) step(() => setCell(tab.edits, p, column, cells[col], value));
    },
    /** Stages the column's DEFAULT for a cell. */
    setDefault(cells, col) {
      this.setValue(cells, col, DEFAULT_VALUE);
    },
    /** Toggles deletion for rows; returns how many rows could be marked. */
    toggleDelete(rowsCells) {
      const all = rowsCells.map(parts).filter(Boolean);
      if (all.length) step(() => toggleDeletes(tab.edits, all));
      return all.length;
    },
    revert(cells) {
      const p = parts(cells);
      if (p) step(() => revertRow(tab.edits, p));
    },
    addRow(values) {
      step(() => tab.edits.inserts.push(values));
    },
    /**
     * Stages copies of rows as new rows. Key columns are left out so the
     * table assigns them (or the user fills them in the review).
     */
    duplicate(rowsCells) {
      const keys = new Set(primaryKey.value);
      const copies = rowsCells.filter(Boolean).map((cells) =>
        Object.fromEntries(
          columns.value
            .map((column, i) => [column.name, cells[i]])
            .filter(([n, cell]) => !keys.has(n) && cell?.kind !== "binary")
            .map(([n, cell]) => [
              n,
              cell?.kind === "value" ? cell.value : null,
            ]),
        ),
      );
      if (copies.length) step(() => tab.edits.inserts.push(...copies));
      return copies.length;
    },
    /** Stages pasted TSV/CSV lines as new rows; returns how many. */
    paste(text) {
      const rows = parsePastedRows(
        text,
        columns.value.map((c) => c.name),
      );
      if (rows.length) step(() => tab.edits.inserts.push(...rows));
      return rows.length;
    },
    canUndo: computed(() => (void version.value, history.past.length > 0)),
    canRedo: computed(() => (void version.value, history.future.length > 0)),
    undo: () => travel(history.past, history.future),
    redo: () => travel(history.future, history.past),
    discard() {
      step(() => (tab.edits = createEdits()));
    },
    /** After the edits were applied: nothing staged, nothing to undo. */
    reset() {
      tab.edits = createEdits();
      history.past = [];
      history.future = [];
      version.value += 1;
    },
    changes: () => toChanges(tab.edits),
  };
}

/**
 * Staged edits for a table's rows, kept until the user applies or discards
 * them. Rows are identified by their primary key, so edits survive sorting
 * and paging. Pure functions over a plain object, so the state can live on
 * the tab and be unit-tested.
 *
 * edits = {
 *   updates: { [rowKey]: { key: [{column, value}], set: { [column]: value|null } } },
 *   deletes: { [rowKey]: { key } },
 *   inserts: [{ [column]: value }],
 * }
 */

export function createEdits() {
  return { updates: {}, deletes: {}, inserts: [] };
}

/** The row's key parts, or null when a key cell isn't plain text. */
export function keyParts(cells, columns, pkIndexes) {
  if (!cells || !pkIndexes.length) return null;
  const parts = [];
  for (const i of pkIndexes) {
    const cell = cells[i];
    if (cell?.kind !== "value") return null;
    parts.push({ column: columns[i].name, value: cell.value });
  }
  return parts;
}

export function rowKey(parts) {
  return parts ? JSON.stringify(parts.map((p) => p.value)) : null;
}

/** Text value of a cell for editing, or null for NULL. */
export function cellValue(cell) {
  return cell?.kind === "value" ? cell.value : null;
}

/** Stages `value` (string, or null for NULL) for one cell. */
export function setCell(edits, parts, column, original, value) {
  const key = rowKey(parts);
  const row = edits.updates[key] ?? { key: parts, set: {} };
  if (value === cellValue(original) && original?.kind !== "binary")
    delete row.set[column];
  else row.set[column] = value;
  if (Object.keys(row.set).length) edits.updates[key] = row;
  else delete edits.updates[key];
}

/** Marks rows for deletion, or unmarks them if all were marked. */
export function toggleDeletes(edits, rowsParts) {
  const keys = rowsParts.map(rowKey);
  const allMarked = keys.every((key) => edits.deletes[key]);
  rowsParts.forEach((parts, i) => {
    if (allMarked) delete edits.deletes[keys[i]];
    else edits.deletes[keys[i]] = { key: parts };
  });
}

export function revertRow(edits, parts) {
  const key = rowKey(parts);
  delete edits.updates[key];
  delete edits.deletes[key];
}

/** The cell to show: the staged value if there is one. */
export function shownCell(edits, key, column, cell) {
  const set = edits.updates[key]?.set;
  if (!set || !(column in set)) return cell;
  return set[column] === null
    ? { kind: "null" }
    : { kind: "value", value: set[column] };
}

export function counts(edits) {
  const deleted = Object.keys(edits.deletes).length;
  // An edited row that is also deleted only counts as deleted.
  const edited = Object.keys(edits.updates).filter(
    (key) => !edits.deletes[key],
  ).length;
  const inserted = edits.inserts.length;
  return { edited, deleted, inserted, total: edited + deleted + inserted };
}

/** The changes in the shape Rust expects. */
export function toChanges(edits) {
  const changes = [];
  for (const { key } of Object.values(edits.deletes))
    changes.push({ kind: "delete", key });
  for (const [rowId, { key, set }] of Object.entries(edits.updates)) {
    if (edits.deletes[rowId]) continue;
    changes.push({
      kind: "update",
      key,
      set: Object.entries(set).map(([column, value]) => ({ column, value })),
    });
  }
  for (const values of edits.inserts)
    changes.push({
      kind: "insert",
      values: Object.entries(values).map(([column, value]) => ({
        column,
        value,
      })),
    });
  return changes;
}

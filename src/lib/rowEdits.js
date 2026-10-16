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

/** A staged value meaning "the column's DEFAULT", rather than any text. */
export const DEFAULT_VALUE = "\u0000default";

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
  if (set[column] === null) return { kind: "null" };
  if (set[column] === DEFAULT_VALUE) return { kind: "default" };
  return { kind: "value", value: set[column] };
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

/** One column's staged value as Rust takes it. */
function columnValue(column, value) {
  return value === DEFAULT_VALUE
    ? { column, value: null, useDefault: true }
    : { column, value };
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
      set: Object.entries(set).map(([column, value]) =>
        columnValue(column, value),
      ),
    });
  }
  for (const values of edits.inserts)
    changes.push({
      kind: "insert",
      values: Object.entries(values).map(([column, value]) =>
        columnValue(column, value),
      ),
    });
  return changes;
}

/**
 * Rows pasted from the clipboard (TSV from a spreadsheet or this grid, or
 * CSV), as new rows keyed by column name. A first line that repeats the
 * column names is a header and skipped; `NULL` on its own is SQL NULL.
 */
export function parsePastedRows(text, columnNames) {
  const lines = text.replace(/\r\n?/g, "\n").replace(/\n$/, "").split("\n");
  if (!lines.length || (lines.length === 1 && !lines[0])) return [];
  const delimiter = lines[0].includes("\t") ? "\t" : ",";
  const split = (line) =>
    delimiter === "\t" ? line.split("\t") : splitCsvLine(line);
  let rows = lines.map(split);
  const isHeader = rows[0].every(
    (cell, i) => cell.trim() === (columnNames[i] ?? "").trim(),
  );
  if (isHeader) rows = rows.slice(1);
  return rows.map((cells) =>
    Object.fromEntries(
      cells
        .slice(0, columnNames.length)
        .map((cell, i) => [columnNames[i], cell === "NULL" ? null : cell]),
    ),
  );
}

function splitCsvLine(line) {
  const out = [];
  let current = "";
  let quoted = false;
  for (let i = 0; i < line.length; i += 1) {
    const c = line[i];
    if (quoted) {
      if (c === '"' && line[i + 1] === '"') {
        current += '"';
        i += 1;
      } else if (c === '"') quoted = false;
      else current += c;
    } else if (c === '"') quoted = true;
    else if (c === ",") {
      out.push(current);
      current = "";
    } else current += c;
  }
  out.push(current);
  return out;
}

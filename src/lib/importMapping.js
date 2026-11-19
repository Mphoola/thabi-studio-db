/**
 * Matching a file's columns to a table's, for the import dialog.
 */

/** "Customer ID" and "customer_id" are the same column for matching. */
function key(name) {
  return String(name ?? "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

/**
 * For each table column, the file column with the same name, ignoring case,
 * spaces and punctuation, or -1 for none.
 */
export function autoMap(fileColumns, tableColumns) {
  const byKey = new Map(fileColumns.map((name, i) => [key(name), i]));
  return tableColumns.map((column) => byKey.get(key(column)) ?? -1);
}

/** The mapping Rust takes, leaving out table columns with no file column. */
export function toMappings(tableColumns, chosen) {
  return tableColumns
    .map((column, i) => ({ column, fileColumn: chosen[i] }))
    .filter((m) => m.fileColumn >= 0);
}

/** The format a file's name suggests. */
export function formatFor(path) {
  return /\.(json|jsonl|ndjson)$/i.test(path ?? "") ? "json" : "csv";
}

/**
 * Display and text helpers for the results grid. Values arrive as cells:
 * { kind: "null" } | { kind: "value", value } | { kind: "binary", value: { bytes, preview } }.
 */

const NUMERIC =
  /^(int|integer|smallint|bigint|tinyint|mediumint|int[248]|decimal|numeric|real|float|float[48]|double|money|smallmoney|bit|i\d+|u8|f\d+|intn|floatn|decimaln|numericn)\b/i;
const JSON_TYPE = /^(json|jsonb)$/i;

export function isNumericType(typeName) {
  return NUMERIC.test(typeName ?? "");
}

export function isJsonType(typeName) {
  return JSON_TYPE.test(typeName ?? "");
}

/** Short text for a grid cell. */
export function displayText(cell) {
  if (!cell || cell.kind === "null") return "NULL";
  if (cell.kind === "binary")
    return `‹${cell.value.bytes.toLocaleString()} bytes›`;
  // Keep cells one line; the inspector shows the full value.
  return cell.value.length > 500
    ? `${cell.value.slice(0, 500).replace(/\s+/g, " ")}…`
    : cell.value.replace(/[\r\n\t]+/g, " ");
}

/** Full value for the inspector; JSON is pretty-printed when it parses. */
export function inspectText(cell, typeName) {
  if (!cell || cell.kind === "null") return null;
  if (cell.kind === "binary")
    return `${cell.value.bytes.toLocaleString()} bytes\n0x${cell.value.preview}${
      cell.value.bytes > cell.value.preview.length / 2 ? "…" : ""
    }`;
  const text = cell.value;
  const trimmed = text.trim();
  if (isJsonType(typeName) || /^[[{]/.test(trimmed)) {
    try {
      return JSON.stringify(JSON.parse(trimmed), null, 2);
    } catch {
      return text;
    }
  }
  return text;
}

/** Same rules as the Rust side (results.rs) for rows held in the page. */
export function csvField(text) {
  return /[",\n\r]|^ | $/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

export function cellText(cell) {
  if (!cell || cell.kind === "null") return "";
  if (cell.kind === "binary") return `0x${cell.value.preview}`;
  return cell.value;
}

/** Rows as TSV or CSV text, for results held only in the page (previews). */
export function rowsToText(
  columns,
  rows,
  { format = "tsv", includeHeader = false } = {},
) {
  const field =
    format === "csv" ? csvField : (text) => text.replace(/[\t\n\r]/g, " ");
  const separator = format === "csv" ? "," : "\t";
  const lines = rows.map((row) =>
    row.map((cell) => field(cellText(cell))).join(separator),
  );
  if (includeHeader)
    lines.unshift(columns.map((c) => field(c.name)).join(separator));
  return `${lines.join("\n")}\n`;
}

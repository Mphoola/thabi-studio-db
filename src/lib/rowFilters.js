/** Operators for the table filter bar. `needsValue: false` hides the value field. */
export const FILTER_OPERATORS = [
  { value: "eq", label: "=" },
  { value: "ne", label: "≠" },
  { value: "lt", label: "<" },
  { value: "le", label: "≤" },
  { value: "gt", label: ">" },
  { value: "ge", label: "≥" },
  { value: "contains", label: "contains" },
  { value: "not_contains", label: "doesn’t contain" },
  { value: "starts_with", label: "starts with" },
  { value: "ends_with", label: "ends with" },
  { value: "in", label: "in list" },
  { value: "not_in", label: "not in list" },
  { value: "is_null", label: "is NULL", needsValue: false },
  { value: "not_null", label: "is not NULL", needsValue: false },
];

/** The column choice that switches a row to a raw SQL condition. */
export const RAW_SQL = "\u0000raw";

export function needsValue(op) {
  return FILTER_OPERATORS.find((o) => o.value === op)?.needsValue !== false;
}

export function newFilter(column = "") {
  return { column, op: "eq", value: "", enabled: true };
}

/** Enabled, complete filters in the shape Rust expects. */
export function toRequest(filters) {
  return filters
    .filter((f) => f.enabled)
    .filter((f) =>
      f.column === RAW_SQL
        ? f.value.trim()
        : f.column && (!needsValue(f.op) || f.value !== ""),
    )
    .map((f) =>
      f.column === RAW_SQL
        ? { column: "", op: "raw", value: f.value.trim() }
        : { column: f.column, op: f.op, value: f.value },
    );
}

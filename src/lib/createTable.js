/**
 * The Create Table wizard's rules, kept out of the component so they are
 * tested: suggested types per engine, a blank row, and the plan Rust turns
 * into one CREATE TABLE.
 */

/** Common types in each engine's own words. Any other type can be typed. */
export const TYPE_SUGGESTIONS = {
  postgres: [
    "INTEGER",
    "BIGINT",
    "BIGSERIAL",
    "SMALLINT",
    "TEXT",
    "VARCHAR(255)",
    "NUMERIC(12,2)",
    "BOOLEAN",
    "DATE",
    "TIMESTAMPTZ",
    "UUID",
    "JSONB",
    "BYTEA",
  ],
  mysql: [
    "INT",
    "BIGINT",
    "SMALLINT",
    "TEXT",
    "VARCHAR(255)",
    "DECIMAL(12,2)",
    "BOOLEAN",
    "DATE",
    "DATETIME",
    "TIMESTAMP",
    "JSON",
    "BLOB",
  ],
  sqlserver: [
    "INT",
    "BIGINT",
    "SMALLINT",
    "NVARCHAR(255)",
    "NVARCHAR(MAX)",
    "VARCHAR(255)",
    "DECIMAL(12,2)",
    "BIT",
    "DATE",
    "DATETIME2",
    "UNIQUEIDENTIFIER",
    "VARBINARY(MAX)",
  ],
};

export function typesFor(engine) {
  return TYPE_SUGGESTIONS[engine] ?? TYPE_SUGGESTIONS.postgres;
}

/** The rows a new table starts with: a key and one more column. */
export function startingRows(engine) {
  const types = typesFor(engine);
  const text = types.find((t) => /TEXT|NVARCHAR\(MAX\)/.test(t)) ?? types[0];
  return [
    { name: "id", type: types[0], nullable: false, key: true, default: "" },
    blankRow(engine, text),
  ];
}

export function blankRow(engine, type) {
  return {
    name: "",
    type: type ?? typesFor(engine)[0],
    nullable: true,
    key: false,
    default: "",
  };
}

/**
 * What stops the table from being created, or "" when nothing does. Said in
 * the order someone would fix it.
 */
export function createProblem(name, rows) {
  if (!name.trim()) return "Give the table a name.";
  const named = rows.filter((row) => row.name.trim());
  if (!named.length) return "Give the table at least one column.";
  const seen = new Set();
  for (const row of named) {
    const key = row.name.trim().toLowerCase();
    if (seen.has(key))
      return `There are two columns called ${row.name.trim()}.`;
    seen.add(key);
    if (!row.type.trim()) return `Give ${row.name.trim()} a type.`;
  }
  return "";
}

/** The create plan Rust builds the statement from, or null. */
export function createPlan(name, rows) {
  if (createProblem(name, rows)) return null;
  return {
    create: {
      columns: rows
        .filter((row) => row.name.trim())
        .map((row) => ({
          name: row.name.trim(),
          dataType: row.type.trim(),
          // Key columns are never nullable on any engine.
          nullable: row.nullable && !row.key,
          default: row.default.trim(),
          primaryKey: row.key,
        })),
    },
  };
}

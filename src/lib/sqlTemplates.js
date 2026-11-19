/**
 * SELECT, INSERT, UPDATE and DELETE written out for a table, to start a
 * query from. Values are `:name` parameters, which the editor asks for and
 * binds, so running a template as it stands never splices a value into SQL.
 * UPDATE and DELETE are keyed by the primary key; without one they say so
 * rather than touching every row.
 */
import { qualifiedName, quoteIdent } from "./explorerTree.js";

/** A parameter name for a column: letters, digits and underscores. */
export function parameterName(column) {
  const cleaned = column.replace(/\W+/g, "_").replace(/^_+|_+$/g, "");
  return /^[A-Za-z_]/.test(cleaned) ? cleaned : `c_${cleaned}`;
}

function keyColumns(columns) {
  return columns
    .filter((column) => column.primaryKey)
    .sort((a, b) => a.primaryKey - b.primaryKey);
}

function where(engine, keys) {
  return keys
    .map((c) => `${quoteIdent(engine, c.name)} = :${parameterName(c.name)}`)
    .join("\n  AND ");
}

const NO_KEY =
  "-- This table has no primary key, so there is nothing to pick one row by.\n" +
  "-- Write the WHERE clause yourself before running this.\n";

/**
 * @param {"select"|"insert"|"update"|"delete"} kind
 * @param {string} engine
 * @param {{ database?: string, schema?: string, name: string }} node
 * @param {{ name: string, primaryKey?: number|null }[]} columns
 */
export function sqlTemplate(kind, engine, node, columns) {
  const table = qualifiedName(engine, node);
  const names = columns.map((c) => quoteIdent(engine, c.name));
  const keys = keyColumns(columns);
  switch (kind) {
    case "select": {
      const limit =
        engine === "sqlserver"
          ? { top: "TOP 100 ", tail: "" }
          : { top: "", tail: "\nLIMIT 100" };
      return `SELECT ${limit.top}${names.join(",\n       ")}\nFROM ${table}${limit.tail};`;
    }
    case "insert":
      return `INSERT INTO ${table} (${names.join(", ")})\nVALUES (${columns
        .map((c) => `:${parameterName(c.name)}`)
        .join(", ")});`;
    case "update": {
      const set = columns
        .filter((c) => !c.primaryKey)
        .map((c) => `${quoteIdent(engine, c.name)} = :${parameterName(c.name)}`)
        .join(",\n    ");
      if (!keys.length)
        return `${NO_KEY}UPDATE ${table}\nSET ${set}\nWHERE /* condition */;`;
      return `UPDATE ${table}\nSET ${set}\nWHERE ${where(engine, keys)};`;
    }
    case "delete":
      if (!keys.length)
        return `${NO_KEY}DELETE FROM ${table}\nWHERE /* condition */;`;
      return `DELETE FROM ${table}\nWHERE ${where(engine, keys)};`;
    default:
      return "";
  }
}

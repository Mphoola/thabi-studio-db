/**
 * Whether a query's result can be edited like a table's.
 *
 * Editing a result means writing `UPDATE table SET … WHERE key = …`, so three
 * things have to be true and none of them can be assumed:
 *
 * - **One table, plainly read.** A join, a comma of tables, a subquery, a set
 *   operation or an aggregate can all produce rows that are not one row each of
 *   one table. Editing one of them writes to a row the user is not looking at.
 * - **The whole key is in the result.** Without every key column there is no
 *   way to say which row to write.
 * - **No key appears twice.** Two rows with the same key mean the result shows
 *   the same record more than once; editing either edits both, and the user
 *   cannot tell which one they clicked.
 *
 * The text check is deliberately strict: anything it does not recognise is
 * refused rather than guessed at. A refusal costs a read-only result; a wrong
 * "yes" costs someone's data.
 */

import { stripComments } from "./sqlStatements.js";

/**
 * Everything in the statement with string literals blanked out.
 *
 * Only single quotes: `"orders"`, `` `orders` `` and `[orders]` are how the
 * three engines quote an *identifier*, and blanking them would hide the very
 * table name being looked for.
 */
function maskLiterals(text) {
  let out = "";
  for (let i = 0; i < text.length; i += 1) {
    const character = text[i];
    if (character === "'") {
      const quote = character;
      out += quote;
      i += 1;
      while (i < text.length && text[i] !== quote) {
        out += text[i] === "\\" ? "  " : " ";
        if (text[i] === "\\") i += 1;
        i += 1;
      }
      out += quote;
      continue;
    }
    out += character;
  }
  return out;
}

/**
 * The one table a statement reads, or null when it is not that simple.
 *
 * @param {string} statement a single statement, without its trailing `;`
 * @returns {{ table: string, schema: string, alias: string } | null}
 */
export function singleTableSelect(statement) {
  const masked = maskLiterals(stripComments(statement ?? ""));
  if (!/^\s*select\s/i.test(masked)) return null;
  // One statement, as the splitter hands it over. Anything else is not this
  // function's to trust.
  if (masked.includes(";")) return null;
  // A second SELECT is a subquery, which can filter, expand or duplicate rows.
  if ((masked.match(/\bselect\b/gi) ?? []).length > 1) return null;

  // Every selected thing must be a plain column: no call, no parenthesis, no
  // window. That refuses aggregates and expressions in one rule rather than by
  // listing every function each engine has.
  const list = masked.slice(/^\s*select\s/i.exec(masked)[0].length);
  const columnsEnd = /\bfrom\b/i.exec(list);
  if (!columnsEnd) return null;
  const selected = list
    .slice(0, columnsEnd.index)
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean);
  if (!selected.length) return null;
  const isColumn =
    /^[\w$".`[\]-]+(?:\.[\w$".`[\]-]+)*(?:\s+as\s+[\w$"`[\]-]+)?$/i;
  if (!selected.every((part) => isColumn.test(part))) return null;

  // A CTE, a set operation, or anything joining or aggregating rows is out.
  if (
    /\b(with|union|intersect|except|join|group\s+by|having|distinct|over)\b/i.test(
      masked,
    )
  )
    return null;

  const from = /\bfrom\b/i.exec(masked);
  if (!from) return null;

  // Take the FROM clause: up to the keyword that ends it.
  const after = masked.slice(from.index + from[0].length);
  const tail =
    /\b(where|group\s+by|having|order\s+by|limit|offset|fetch|union|for\s+update|window)\b/i.exec(
      after,
    );
  let clause = (tail ? after.slice(0, tail.index) : after).trim();
  if (!clause) return null;

  // Another table in the same FROM, or a subquery standing in for one.
  if (clause.includes(",")) return null;
  if (clause.startsWith("(")) return null;

  // `schema.table alias`, `table AS alias`, or bare `table`.
  const parts = clause.split(/\s+/).filter(Boolean);
  let name = parts[0] ?? "";
  let alias = parts[1] ?? "";
  // `AS` with no name after it is not a table reference this can trust.
  if (alias && alias.toLowerCase() === "as" && !parts[2]) return null;
  if (alias && alias.toLowerCase() === "as") alias = parts[2] ?? "";
  if (parts.length > 3 || (parts.length === 3 && alias.toLowerCase() !== "as"))
    return null;

  const segments = name.split(".");
  const unquoted = (part) =>
    part
      .replace(/^"(.*)"$/s, "$1")
      .replace(/^`(.*)`$/s, "$1")
      .replace(/^\[(.*)\]$/s, "$1");
  const table = unquoted(segments[segments.length - 1] ?? "");
  if (!table || !/^[\w$.-]+$/.test(table)) return null;
  const schema =
    segments.length > 1 ? unquoted(segments[segments.length - 2]) : "";
  return { table, schema, alias: unquoted(alias) };
}

/** The columns a key needs in the result to address a row. */
export function missingKeyColumns(columns, primaryKey) {
  const names = new Set(columns.map((column) => column.name));
  return primaryKey.filter((column) => !names.has(column));
}

/** Whether a result repeats a key, which would make editing ambiguous. */
export function repeatedKeys(columns, primaryKey, rows) {
  const places = primaryKey.map((name) =>
    columns.findIndex((c) => c.name === name),
  );
  if (places.some((place) => place < 0)) return false;
  // A cell is normally `{ kind, value }`; a raw value is accepted too, since
  // this is only ever about which row is which.
  const valueAt = (cell) =>
    cell && typeof cell === "object" ? (cell.value ?? null) : (cell ?? null);
  const seen = new Set();
  for (const cells of rows) {
    const key = JSON.stringify(places.map((place) => valueAt(cells[place])));
    if (seen.has(key)) return true;
    seen.add(key);
  }
  return false;
}

/**
 * Whether a result can be edited, and why not when it can't.
 *
 * @param {object} input
 * @param {string} input.statement the statement that produced the result
 * @param {{name: string}[]} input.columns the result's columns
 * @param {string[]} input.primaryKey the table's key columns, in key order
 * @param {string[][]} input.rows the rows on screen
 * @param {string} [input.database] the database the result came from
 * @returns {{ editable: boolean, reason: string, table?: object, database?: string }}
 */
export function resultEditability({
  statement,
  columns,
  primaryKey = [],
  rows = [],
  database = "",
}) {
  const table = singleTableSelect(statement);
  if (!table)
    return {
      editable: false,
      reason:
        "Only a plain SELECT from one table can be edited. Joins, subqueries, aggregates and set operations cannot.",
    };
  if (!primaryKey.length)
    return {
      editable: false,
      reason: `${table.table} has no primary key, so a row cannot be addressed.`,
      table,
    };
  const missing = missingKeyColumns(columns, primaryKey);
  if (missing.length)
    return {
      editable: false,
      reason: `Add ${missing.join(", ")} to the result so each row can be identified.`,
      table,
    };
  if (repeatedKeys(columns, primaryKey, rows))
    return {
      editable: false,
      reason: `A row appears more than once in this result, so editing one would edit the other.`,
      table,
    };
  return { editable: true, reason: "", table, database };
}

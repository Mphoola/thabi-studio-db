/**
 * Which tables are in scope in a statement, and what they are called there.
 *
 * Typing `o.` in `FROM orders o JOIN customers c` should offer orders'
 * columns, not every column in the schema. So the FROM clause is read for the
 * names the statement itself introduces, and an alias is resolved through them.
 *
 * The reading is deliberately generous about *what* it accepts and careful about
 * what it reports: offering a column that turned out to be absent is a
 * suggestion the user ignores, while missing one is a suggestion that is not
 * there.
 */

import { stripComments } from "./sqlStatements.js";

/** Replaces string literals with spaces, so a `FROM` inside one is not a FROM. */
export function maskStrings(text) {
  let out = "";
  for (let i = 0; i < text.length; i += 1) {
    const character = text[i];
    if (character === "'") {
      out += " ";
      i += 1;
      while (i < text.length) {
        if (text[i] === "'") {
          // `''` is an escaped quote, so the literal carries on.
          if (text[i + 1] === "'") {
            i += 2;
            continue;
          }
          break;
        }
        out += " ";
        i += 1;
      }
      continue;
    }
    out += character;
  }
  return out;
}

/** The identifier a quoted name stands for. */
function unquote(name) {
  return name
    .replace(/^"(.*)"$/s, "$1")
    .replace(/^`(.*)`$/s, "$1")
    .replace(/^\[(.*)\]$/s, "$1");
}

/** Whether a word is a keyword that ends a table reference. */
const NOT_A_NAME = new Set([
  "on",
  "where",
  "group",
  "order",
  "having",
  "limit",
  "offset",
  "union",
  "intersect",
  "except",
  "inner",
  "left",
  "right",
  "full",
  "outer",
  "cross",
  "join",
  "using",
  "natural",
  "window",
  "fetch",
  "for",
  "set",
  "returning",
]);

/**
 * Splits a clause into words, keeping a quoted name whole.
 *
 * `"Order Items"` is one name with a space in it, and splitting on whitespace
 * would read it as two.
 */
function words(text) {
  const parts = [];
  let current = "";
  for (let i = 0; i < text.length; i += 1) {
    const character = text[i];
    const close =
      character === '"'
        ? '"'
        : character === "`"
          ? "`"
          : character === "["
            ? "]"
            : null;
    if (close) {
      let name = character;
      i += 1;
      while (i < text.length && text[i] !== close) {
        name += text[i];
        i += 1;
      }
      current += name + close;
      continue;
    }
    if (/\s/.test(character)) {
      if (current) parts.push(current);
      current = "";
      continue;
    }
    current += character;
  }
  if (current) parts.push(current);
  return parts;
}

/** A table reference: `table`, `schema.table`, `schema.table AS alias`. */
function readReference(text) {
  const parts = words(text);
  if (!parts.length) return null;
  // `JOIN other ON ...` — only the name and what follows it are ours. The name
  // is left as written: `` `shop`.`orders` `` is one word made of two quoted
  // names, and unquoting the outside first leaves the inside unbalanced.
  const name = parts[0];
  if (!name) return null;
  let alias = "";
  if (parts[1] && !NOT_A_NAME.has(parts[1].toLowerCase())) {
    alias =
      parts[1].toLowerCase() === "as"
        ? parts[2]
          ? unquote(parts[2])
          : ""
        : unquote(parts[1]);
  }
  return { table: name, alias };
}

/**
 * Every table the statement introduces, with the name it is called there.
 *
 * @param {string} sql the statement, or the text before the cursor
 * @returns {{ table: string, alias: string, schema: string }[]}
 */
export function statementTables(sql) {
  const masked = maskStrings(stripComments(sql ?? ""));
  const found = [];
  // Each FROM and each JOIN introduces a table. A comma list inside one FROM
  // introduces several.
  const clauses = masked.match(/\b(?:from|join)\b/gi) ?? [];
  for (const keyword of clauses) {
    const at = masked.indexOf(keyword, found.at(-1)?.end ?? 0);
    if (at === -1) continue;
    let end = at + keyword.length;
    const rest = masked.slice(end);
    const stop =
      /\b(on|where|group\s+by|order\s+by|having|union|intersect|except|join|inner|left|right|full|cross|natural|using|window|fetch|for|returning|limit|offset)\b/i.exec(
        rest,
      );
    const clause = stop ? rest.slice(0, stop.index) : rest;
    end += clause.length;
    for (const piece of clause.split(",")) {
      const reference = readReference(piece);
      if (reference) found.push({ ...reference, schema: "", end });
    }
  }
  // A qualified name gives the schema away: `public.orders`. Each part keeps
  // whatever quoting it had, since `` `shop`.`orders` `` is two quoted names.
  return found.map(({ table, alias }) => {
    const parts = table.split(".");
    return {
      table: unquote(parts.at(-1) ?? ""),
      schema: parts.length > 1 ? unquote(parts.at(-2) ?? "") : "",
      alias,
    };
  });
}

/**
 * The table a qualifier refers to in the statement, if it is one.
 *
 * @param {{ table: string, alias: string }[]} tables from `statementTables`
 * @param {string} qualifier what was typed before the dot, e.g. `o`
 * @returns {{ table: string, alias: string } | null}
 */
export function resolveQualifier(tables, qualifier) {
  const wanted = unquote(qualifier.trim().toLowerCase());
  if (!wanted) return null;
  // An alias is what the statement calls it; the table name works too, so
  // `orders.` still works in `FROM orders o`.
  return (
    tables.find((entry) => entry.alias.toLowerCase() === wanted) ??
    tables.find((entry) => entry.table.toLowerCase() === wanted) ??
    null
  );
}

/**
 * What a qualified prefix refers to: an alias, or a column of a table.
 *
 * @param {{ table: string, alias: string }[]} tables from `statementTables`
 * @param {string} prefix what was typed, e.g. `o.` or `orders.na`
 */
export function qualifyPrefix(tables, prefix) {
  const dot = prefix.lastIndexOf(".");
  if (dot === -1) return { qualifier: "", rest: prefix };
  return {
    qualifier: prefix.slice(0, dot),
    rest: prefix.slice(dot + 1),
  };
}

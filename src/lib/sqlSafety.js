import { stripComments } from "./sqlStatements.js";

/**
 * A rough reading of what a statement does, used only to ask before running
 * something risky. SQL is too varied to analyse reliably here, so this is a
 * safeguard against common mistakes, not a security boundary.
 */

const READ_ONLY = new Set([
  "SELECT",
  "SHOW",
  "EXPLAIN",
  "DESCRIBE",
  "DESC",
  "VALUES",
  "TABLE",
  "USE",
  "SET",
  "BEGIN",
  "START",
  "COMMIT",
  "ROLLBACK",
  "SAVEPOINT",
  "RELEASE",
  "PRAGMA",
]);

// An identifier, possibly quoted and qualified: a.b, "a b"."c", [x].[y].
const PART = '(?:"[^"]+"|`[^`]+`|\\[[^\\]]+\\]|[^\\s(;,."`\\[]+)';
const IDENT = `(${PART}(?:\\.${PART})*)`;

/**
 * @param {string} statement
 * @returns {{ changesData: boolean, risk: null | { kind: "no-where" | "drop" | "truncate", verb: string, target: string } }}
 */
export function classifyStatement(statement) {
  // Blank out string literals so keywords inside them don't count.
  const sql = stripComments(statement)
    .replace(/'(?:[^']|'')*'/g, "''")
    .replace(/\s+/g, " ")
    .trim();
  const upper = sql.toUpperCase();
  const first = upper.split(/[\s(]/, 1)[0];

  const changesData =
    first === "WITH"
      ? /\b(INSERT|UPDATE|DELETE|MERGE)\b/.test(upper)
      : !READ_ONLY.has(first) ||
        // SELECT … INTO new_table creates a table.
        (first === "SELECT" && /\bINTO\s+(?!@|OUTFILE|DUMPFILE)/.test(upper));

  return { changesData, risk: findRisk(upper, sql) };
}

function unquote(name) {
  return (name.match(new RegExp(PART, "g")) ?? [])
    .map((part) => part.replace(/^["`[]|["`\]]$/g, ""))
    .join(".");
}

function target(sql, prefix) {
  const match = new RegExp(`^${prefix}\\s*${IDENT}`, "i").exec(sql);
  return match ? unquote(match[1]) : "";
}

function findRisk(upper, sql) {
  if (/^(UPDATE|DELETE)\b/.test(upper) && !/\bWHERE\b/.test(upper)) {
    const update = upper.startsWith("UPDATE");
    return {
      kind: "no-where",
      verb: update ? "UPDATE" : "DELETE",
      target: update
        ? target(sql, "update\\s+(?:only\\s+)?")
        : target(sql, "delete\\s+(?:from\\s+)?"),
    };
  }
  if (/^DROP\s+(TABLE|DATABASE|SCHEMA|VIEW|INDEX)\b/.test(upper))
    return {
      kind: "drop",
      verb: "DROP",
      target: target(sql, "drop\\s+\\w+\\s+(?:if\\s+exists\\s+)?"),
    };
  if (/^TRUNCATE\b/.test(upper))
    return {
      kind: "truncate",
      verb: "TRUNCATE",
      target: target(sql, "truncate\\s+(?:table\\s+)?"),
    };
  return null;
}

/**
 * What to ask before running, or null to run straight away.
 * @param {string[]} statements
 * @param {{ environment: string, database: string | null, connection: string }} context
 */
export function confirmationFor(statements, context) {
  const classified = statements.map(classifyStatement);
  const risky = classified.find((c) => c.risk)?.risk;
  if (risky?.kind === "no-where") {
    const action = risky.verb === "UPDATE" ? "Update" : "Delete";
    return {
      title: `${action} every row in “${risky.target}”?`,
      message: `This ${risky.verb} has no WHERE clause.`,
      confirmLabel: `${action} All Rows`,
      destructive: true,
    };
  }
  if (risky) {
    const drop = risky.kind === "drop";
    return {
      title: `${drop ? "Drop" : "Empty"} “${risky.target}”?`,
      message: drop
        ? "This removes it and everything in it. It can't be undone."
        : "This deletes every row. It can't be undone.",
      confirmLabel: drop ? "Drop" : "Truncate",
      destructive: true,
    };
  }
  if (
    context.environment === "production" &&
    classified.some((c) => c.changesData)
  ) {
    const where = context.database ? `“${context.database}”` : "this database";
    return {
      title: "Run on Production?",
      message: `This statement can change data in ${where} on “${context.connection}”.`,
      confirmLabel: "Run",
      destructive: true,
    };
  }
  return null;
}

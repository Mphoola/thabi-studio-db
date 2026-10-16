import { stripComments } from "./sqlStatements.js";

/**
 * A rough reading of what a statement does, used only to ask before running
 * something risky. SQL is too varied to analyse reliably here, so this is a
 * safeguard against common mistakes, not a security boundary.
 */

/** The four safe modes a connection can be set to (plan.md §7b, Phase 11). */
export const SAFE_MODES = [
  {
    value: "off",
    label: "Off",
    hint: "Runs statements without asking. Destructive statements still ask.",
  },
  {
    value: "confirm-writes",
    label: "Confirm Writes",
    hint: "Asks before a statement can change data or schema, and before saving edits.",
  },
  {
    value: "confirm-everything",
    label: "Confirm Everything",
    hint: "Asks before every statement runs, including plain SELECTs.",
  },
  {
    value: "read-only",
    label: "Read-only",
    hint: "Refuses anything that can change data or schema — in Thabi Studio, and on the server too on MySQL and PostgreSQL. SQL Server has no session setting, so there only Thabi Studio stops it.",
  },
];

/** The mode a new profile starts on: Production asks, everything else doesn't. */
export function defaultSafeMode(environment) {
  return environment === "production" ? "confirm-writes" : "off";
}

export function safeModeFor(value) {
  return SAFE_MODES.find((mode) => mode.value === value) ?? SAFE_MODES[0];
}

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

/** True when any statement can change data or schema. */
export function changesData(statements) {
  return statements.map(classifyStatement).some((c) => c.changesData);
}

/**
 * Why the statements can't run at all, or null when they can. A read-only
 * connection refuses anything that could change data or schema; the server
 * enforces the same rule where it can, but this is what holds on SQL Server.
 *
 * @param {string[]} statements
 * @param {string} safeMode
 * @returns {string | null}
 */
export function writeBlockFor(statements, safeMode) {
  if (safeMode !== "read-only") return null;
  return changesData(statements)
    ? "This connection is read-only. Statements that can change data or schema were not run."
    : null;
}

/**
 * What to ask before running, or null to run straight away.
 *
 * A destructive statement asks in every mode except read-only, which refuses
 * it outright; on Production the database name has to be typed to confirm.
 * Otherwise the safe mode decides: confirm-everything asks about every
 * statement, confirm-writes only about the ones that change data.
 *
 * @param {string[]} statements
 * @param {{ safeMode?: string, environment?: string, database: string | null, connection: string }} context
 */
export function confirmationFor(statements, context) {
  const {
    safeMode = "off",
    environment = "local",
    database,
    connection,
  } = context;
  const classified = statements.map(classifyStatement);
  const risky = classified.find((c) => c.risk)?.risk;
  // Naming the database is what makes a Production confirmation deliberate.
  const requireText = environment === "production" ? database : null;

  if (risky?.kind === "no-where") {
    const action = risky.verb === "UPDATE" ? "Update" : "Delete";
    return {
      title: `${action} every row in “${risky.target}”?`,
      message: `This ${risky.verb} has no WHERE clause.`,
      confirmLabel: `${action} All Rows`,
      destructive: true,
      requireText,
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
      requireText,
    };
  }

  const where = database ? `“${database}”` : "this database";
  if (safeMode === "confirm-everything") {
    return {
      title: "Run this statement?",
      message: `“${connection}” asks before every statement runs. It runs in ${where}.`,
      confirmLabel: "Run",
    };
  }
  if (safeMode === "confirm-writes" && classified.some((c) => c.changesData)) {
    return {
      title:
        environment === "production"
          ? "Run on Production?"
          : "Run this statement?",
      message: `This statement can change data in ${where} on “${connection}”.`,
      confirmLabel: "Run",
      destructive: true,
    };
  }
  return null;
}

/**
 * The question to ask before applying staged changes to a table, or null to
 * apply straight away. Follows the connection's safe mode the same way
 * statements do, and always asks before deleting rows.
 *
 * @param {{ total: number, deleted: number }} summary
 * @param {{ safeMode?: string, environment?: string, connection: string, table: string }} context
 */
export function applyConfirmationFor(summary, context) {
  const {
    safeMode = "off",
    environment = "local",
    connection,
    table,
  } = context;
  const { total, deleted } = summary;
  if (
    safeMode === "confirm-everything" ||
    (safeMode === "confirm-writes" && !deleted)
  ) {
    return {
      title: `Apply ${total} ${total === 1 ? "change" : "changes"} to “${table}”?`,
      message: `They run in one transaction on “${connection}”.`,
      confirmLabel: "Apply Changes",
      destructive: deleted > 0,
    };
  }
  if (deleted) {
    return {
      title: `Delete ${deleted} ${deleted === 1 ? "row" : "rows"} from “${table}”?`,
      message: `All staged changes are applied together on “${connection}”. This can't be undone.`,
      confirmLabel: "Delete and Apply",
      destructive: true,
      requireText: environment === "production" ? table : null,
    };
  }
  return null;
}

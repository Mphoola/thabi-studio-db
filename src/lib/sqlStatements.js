/**
 * Splits SQL text into statements without executing or fully parsing it.
 * Understands quotes, comments, PostgreSQL dollar quoting, MySQL backticks,
 * SQL Server brackets and SQL Server's `GO` batch separator lines. It is a
 * scanner, not a parser: good enough to find statement boundaries.
 *
 * @typedef {{ start: number, end: number, text: string }} Statement
 */

/**
 * @param {string} sql
 * @param {"postgres" | "mysql" | "sqlserver"} engine
 * @returns {Statement[]}
 */
export function splitStatements(sql, engine = "postgres") {
  const statements = [];
  let start = 0;
  let i = 0;
  const n = sql.length;

  const push = (end) => {
    const raw = sql.slice(start, end);
    const text = raw.trim();
    if (text && !isOnlyComments(text)) {
      const offset = raw.indexOf(text[0]);
      statements.push({
        start: start + offset,
        end: start + offset + text.length,
        text,
      });
    }
  };

  while (i < n) {
    const ch = sql[i];
    const next = sql[i + 1];

    if (ch === "-" && next === "-") {
      i = lineEnd(sql, i);
    } else if (ch === "#" && engine === "mysql") {
      i = lineEnd(sql, i);
    } else if (ch === "/" && next === "*") {
      const close = sql.indexOf("*/", i + 2);
      i = close === -1 ? n : close + 2;
    } else if (ch === "'" || ch === '"' || (ch === "`" && engine === "mysql")) {
      i = quoteEnd(sql, i, ch, engine === "mysql");
    } else if (ch === "[" && engine === "sqlserver") {
      i = quoteEnd(sql, i, "]", false);
    } else if (ch === "$" && engine === "postgres") {
      const tag = /^\$[A-Za-z_]*\$/.exec(sql.slice(i));
      if (tag) {
        const close = sql.indexOf(tag[0], i + tag[0].length);
        i = close === -1 ? n : close + tag[0].length;
      } else i += 1;
    } else if (ch === ";") {
      push(i);
      start = i + 1;
      i += 1;
    } else if (
      engine === "sqlserver" &&
      (ch === "G" || ch === "g") &&
      atLineStart(sql, i) &&
      /^go\s*(?:--.*)?$/i.test(sql.slice(i, lineEnd(sql, i)).trim())
    ) {
      push(i);
      i = lineEnd(sql, i);
      start = i;
    } else {
      i += 1;
    }
  }
  push(n);
  return statements;
}

/**
 * The statement under the cursor; between statements, the one just before.
 * @param {Statement[]} statements
 * @param {number} position
 */
export function statementAt(statements, position) {
  let best = null;
  for (const statement of statements) {
    if (statement.start <= position) best = statement;
    if (position <= statement.end) break;
  }
  return best ?? statements[0] ?? null;
}

function lineEnd(sql, i) {
  const end = sql.indexOf("\n", i);
  return end === -1 ? sql.length : end;
}

function atLineStart(sql, i) {
  let j = i - 1;
  while (j >= 0 && (sql[j] === " " || sql[j] === "\t")) j -= 1;
  return j < 0 || sql[j] === "\n";
}

/** Index just past the closing quote. Doubled quotes are escapes. */
function quoteEnd(sql, i, close, backslashEscapes) {
  let j = i + 1;
  while (j < sql.length) {
    if (backslashEscapes && sql[j] === "\\") j += 2;
    else if (sql[j] === close) {
      if (sql[j + 1] === close) j += 2;
      else return j + 1;
    } else j += 1;
  }
  return sql.length;
}

function isOnlyComments(text) {
  return stripComments(text).trim() === "";
}

/** Removes comments (keeps strings). Used for classification. */
export function stripComments(sql) {
  return sql
    .replace(/\/\*[\s\S]*?\*\//g, " ")
    .replace(/--[^\n]*/g, " ")
    .replace(/^\s*#[^\n]*/gm, " ");
}

/**
 * The database a `USE name` statement switches to (MySQL, SQL Server), or
 * null. Quotes are removed: USE `shop`, USE [shop], USE "shop".
 * @param {string} statement
 */
export function useTarget(statement) {
  const match = /^\s*use\s+(`[^`]+`|\[[^\]]+\]|"[^"]+"|[^\s;]+)\s*;?\s*$/i.exec(
    stripComments(statement),
  );
  return match ? match[1].replace(/^[`["]|[`\]"]$/g, "") : null;
}

const LANGUAGES = {
  postgres: "postgresql",
  mysql: "mysql",
  sqlserver: "transactsql",
};

/**
 * Beautifies SQL for the engine's dialect. Throws when the formatter can't
 * parse the text, so the caller can leave the editor untouched. The
 * formatter is large, so it is loaded the first time it is used.
 */
export async function formatSql(
  text,
  engine,
  { keywordCase = "upper", tabWidth = 2 } = {},
) {
  const { format } = await import("sql-formatter");
  return format(text, {
    language: LANGUAGES[engine] ?? "sql",
    keywordCase,
    tabWidth,
    // Keep placeholders the user typed (:name, ?, $1) as they are.
    paramTypes: { named: [":"], positional: true, numbered: ["$"] },
  });
}

/**
 * Puts SQL on as few lines as possible without changing what it means:
 * whitespace collapses outside strings, quoted names and comments, and a
 * line comment becomes a block comment so joining lines can't swallow code.
 */
export function minifySql(text) {
  let out = "";
  let pendingSpace = false;
  const push = (piece) => {
    if (pendingSpace && out && !/\s$/.test(out)) out += " ";
    pendingSpace = false;
    out += piece;
  };
  for (let i = 0; i < text.length;) {
    const c = text[i];
    if (/\s/.test(c)) {
      pendingSpace = true;
      i += 1;
      continue;
    }
    if (c === "-" && text[i + 1] === "-") {
      const end = text.indexOf("\n", i);
      const body = text.slice(i + 2, end === -1 ? text.length : end).trim();
      push(`/* ${body.replaceAll("*/", "* /")} */`);
      i = end === -1 ? text.length : end;
      continue;
    }
    if (c === "/" && text[i + 1] === "*") {
      const end = text.indexOf("*/", i + 2);
      const stop = end === -1 ? text.length : end + 2;
      push(text.slice(i, stop));
      i = stop;
      continue;
    }
    if (c === "'" || c === '"' || c === "`" || c === "[") {
      const close = c === "[" ? "]" : c;
      let end = i + 1;
      while (end < text.length) {
        if (text[end] === close) {
          if (close !== "]" && text[end + 1] === close) {
            end += 2;
            continue;
          }
          break;
        }
        end += 1;
      }
      push(text.slice(i, end + 1));
      i = end + 1;
      continue;
    }
    push(c);
    i += 1;
  }
  return out.trim();
}

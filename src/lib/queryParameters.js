/**
 * Placeholders in a statement, and the statement rewritten to bind them.
 *
 * A parameter's value is never put into the SQL text. The placeholder is
 * replaced by the engine's own — `$1` on PostgreSQL, `?` on MySQL, `@p1` on
 * SQL Server — and the values travel beside the statement and are bound by the
 * driver. So a value containing `'` or `;` is stored as those characters rather
 * than becoming part of the statement, which is the whole point of having
 * parameters.
 *
 * Detection skips what is not code: string literals, quoted identifiers
 * (which the three engines spell three ways) and both comment styles. `:name`
 * inside a string is text; `x::int` is a cast, not a parameter.
 */

/** Where a placeholder stands, so the form can name it and remember its value. */
const IDENTIFIER = /[\w$]/;

/**
 * The placeholders in a statement, in the order the driver will bind them.
 *
 * @param {string} sql
 * @param {"postgres"|"mysql"|"sqlserver"} engine
 * @returns {{ count: number, names: (string|number)[], sql: string } | null} null
 *   when there is nothing to ask for
 */
export function findParameters(sql, engine = "postgres") {
  const source = sql ?? "";
  const names = [];
  let out = "";
  let index = 0;
  // `postgres` writes $1, `mysql` writes ?, `sqlserver` writes @p1.
  const placeholder = (position) => {
    if (engine === "postgres") return `$${position + 1}`;
    if (engine === "mysql") return "?";
    return `@p${position + 1}`;
  };

  while (index < source.length) {
    const character = source[index];

    // A quoted identifier: not code, and may hold anything.
    if (character === '"' || character === "`" || character === "[") {
      const close = character === "[" ? "]" : character;
      const end = source.indexOf(close, index + 1);
      const stop = end === -1 ? source.length : end + 1;
      out += source.slice(index, stop);
      index = stop;
      continue;
    }

    // A string literal: not code.
    if (character === "'" || character === '"' || character === "`") {
      const quote = character;
      out += quote;
      index += 1;
      while (index < source.length) {
        if (source[index] === quote) {
          // Two quotes in a row is an escaped quote, not the end.
          if (source[index + 1] === quote) {
            out += quote + quote;
            index += 2;
            continue;
          }
          out += quote;
          index += 1;
          break;
        }
        out += source[index];
        index += 1;
      }
      continue;
    }

    // A line comment.
    if (character === "-" && source[index + 1] === "-") {
      const end = source.indexOf("\n", index);
      const stop = end === -1 ? source.length : end;
      out += source.slice(index, stop);
      index = stop;
      continue;
    }

    // A block comment.
    if (character === "/" && source[index + 1] === "*") {
      const end = source.indexOf("*/", index + 2);
      const stop = end === -1 ? source.length : end + 2;
      out += source.slice(index, stop);
      index = stop;
      continue;
    }

    // `value::int` is a cast. `::` is not a placeholder.
    if (character === ":" && source[index + 1] === ":") {
      out += "::";
      index += 2;
      continue;
    }

    if (character === "?") {
      names.push(names.length + 1);
      out += placeholder(names.length - 1);
      index += 1;
      continue;
    }

    if (character === ":") {
      let next = index + 1;
      if (source[next] === ":") {
        out += "::";
        index = next + 1;
        continue;
      }
      if (!IDENTIFIER.test(source[next] ?? "")) {
        out += character;
        index += 1;
        continue;
      }
      let end = next;
      while (end < source.length && IDENTIFIER.test(source[end])) end += 1;
      names.push(source.slice(next, end));
      out += placeholder(names.length - 1);
      index = end;
      continue;
    }

    if (character === "@") {
      let next = index + 1;
      if (!IDENTIFIER.test(source[next] ?? "")) {
        out += character;
        index += 1;
        continue;
      }
      let end = next;
      while (end < source.length && IDENTIFIER.test(source[end])) end += 1;
      names.push(source.slice(next, end));
      out += placeholder(names.length - 1);
      index = end;
      continue;
    }

    // `$1`, `$2` — already PostgreSQL's own positional form.
    if (character === "$" && /[1-9]/.test(source[index + 1] ?? "")) {
      let end = index + 1;
      while (end < source.length && /[0-9]/.test(source[end])) end += 1;
      const position = Number(source.slice(index + 1, end));
      names.push(position);
      out += placeholder(position - 1);
      index = end;
      continue;
    }

    out += character;
    index += 1;
  }

  if (!names.length) return null;
  return { count: names.length, names, sql: out };
}

/**
 * The labels to show for each parameter, in binding order.
 *
 * A positional placeholder has no name of its own, so it is called by its
 * position; `:status` is called `status`.
 */
export function parameterLabels(names) {
  return names.map((name) =>
    typeof name === "number" ? `Parameter ${name}` : name,
  );
}

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

const IDENTIFIER = /[\w$]/;
const NAME_START = /[A-Za-z_]/;

/**
 * The placeholders in a statement, and the statement rewritten to bind them.
 *
 * Recognised: `:name` everywhere, `$1` on PostgreSQL, and `?` on MySQL and
 * SQL Server. `@name` is deliberately not a placeholder: it is how MySQL user
 * variables and T-SQL variables are written (`@@ROWCOUNT` too), and treating
 * them as parameters would break working SQL. `?` is not one on PostgreSQL,
 * where it is a JSON operator.
 *
 * A name used twice is asked for once. On PostgreSQL every use shares one
 * `$n`; on MySQL and SQL Server each use is its own marker and `order` says
 * which value goes where.
 *
 * @param {string} sql
 * @param {"postgres"|"mysql"|"sqlserver"} engine
 * @returns {{ count: number, names: (string|number)[], order: number[], sql: string } | null}
 *   `names` are the distinct parameters to ask for; `order` lists, per value
 *   the driver binds, the index into `names`. null when there is nothing.
 */
export function findParameters(sql, engine = "postgres") {
  const source = sql ?? "";
  const names = [];
  const order = [];
  let out = "";
  let index = 0;

  const use = (name) => {
    let position = names.indexOf(name);
    if (position === -1) {
      names.push(name);
      position = names.length - 1;
    }
    if (engine === "postgres") {
      // One marker per distinct name, reused.
      if (!order.includes(position)) order.push(position);
      return `$${order.indexOf(position) + 1}`;
    }
    order.push(position);
    return engine === "mysql" ? "?" : `@P${order.length}`;
  };

  while (index < source.length) {
    const character = source[index];

    // A quoted identifier or string: not code, and may hold anything.
    if (
      character === '"' ||
      character === "`" ||
      character === "[" ||
      character === "'"
    ) {
      const close = character === "[" ? "]" : character;
      let end = index + 1;
      while (end < source.length) {
        if (source[end] === close) {
          // Two quotes in a row is an escaped quote, not the end.
          if (close !== "]" && source[end + 1] === close) {
            end += 2;
            continue;
          }
          break;
        }
        end += 1;
      }
      const stop = Math.min(source.length, end + 1);
      out += source.slice(index, stop);
      index = stop;
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

    // `value::int` is a cast and `:=` an assignment; neither is a placeholder.
    if (character === ":" && source[index + 1] === ":") {
      out += "::";
      index += 2;
      continue;
    }

    if (character === ":" && NAME_START.test(source[index + 1] ?? "")) {
      // `a[1:n]` slices and `12:30` are not placeholders: a name can't follow
      // a word character or `]`.
      const before = source[index - 1] ?? "";
      if (!/[\w\]]/.test(before)) {
        let end = index + 1;
        while (end < source.length && IDENTIFIER.test(source[end])) end += 1;
        out += use(source.slice(index + 1, end));
        index = end;
        continue;
      }
    }

    if (character === "?" && engine !== "postgres") {
      out += use(order.length + 1);
      index += 1;
      continue;
    }

    // `$1`, `$2` — PostgreSQL's own positional form.
    if (
      engine === "postgres" &&
      character === "$" &&
      /[1-9]/.test(source[index + 1] ?? "") &&
      !/[\w$]/.test(source[index - 1] ?? "")
    ) {
      let end = index + 1;
      while (end < source.length && /[0-9]/.test(source[end])) end += 1;
      out += use(Number(source.slice(index + 1, end)));
      index = end;
      continue;
    }

    out += character;
    index += 1;
  }

  if (!names.length) return null;
  return { count: names.length, names, order, sql: out };
}

/**
 * The values to send, in binding order, from the form's answers per name.
 * Each answer is { type: "text" | "number" | "null", value }.
 */
export function boundValues(found, answers) {
  return found.order.map((position) => {
    const answer = answers[position] ?? { type: "text", value: "" };
    if (answer.type === "null") return { kind: "null" };
    if (answer.type === "number")
      return { kind: "number", value: String(answer.value).trim() };
    return { kind: "text", value: String(answer.value ?? "") };
  });
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

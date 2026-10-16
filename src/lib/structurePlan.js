/**
 * Turns a person's edits to a table into the changes the designer stages.
 *
 * The Structure tab starts from what `describe` reported, so an edit is
 * recorded as "this column, changed like this" rather than as the table it
 * should become. That is what lets one change be one statement — renaming a
 * column and retyping it stays one change, which on SQL Server is two
 * statements the server needs together.
 *
 * Nothing here talks to a database. The result is the plan the review dialog
 * shows and the apply path runs, from one call.
 */

/** One staged change, as the Structure tab records it. */
const edit = (kind, fields) => ({ kind, ...fields });

/**
 * The plan for a list of edits.
 *
 * A rename keeps the type and nullability the column had, because the generated
 * statement restates the whole column on two of the three engines and a type
 * that went missing would be worse than a rename that changed nothing else.
 *
 * @param {object} structure what `describe` reported
 * @param {object[]} edits staged changes, in the order they were made
 * @returns the plan `preview_table_ddl` takes
 */
export function toPlan(structure, edits) {
  const columns = [];
  const keys = [];
  const indexes = [];
  const dropped = new Set();

  // A rename renames the column for everything after it. Without this, a
  // batch that renames a column and then retypes it sends the old name to a
  // table that no longer has one, and the second statement fails.
  const renamed = new Map();
  const currentName = (name) => renamed.get(name) ?? name;

  // Renames first, so the rest of the batch names columns that exist.
  const ordered = [
    ...edits.filter((one) => one.kind === "renameColumn"),
    ...edits.filter((one) => one.kind !== "renameColumn"),
  ];
  for (const one of ordered) {
    if (one.kind === "renameColumn") renamed.set(one.from, one.to);
  }

  for (const one of ordered) {
    switch (one.kind) {
      case "addColumn":
        columns.push({ addColumn: one.column });
        break;
      case "dropColumn":
        dropped.add(one.name);
        columns.push({ dropColumn: { name: one.name } });
        break;
      case "renameColumn": {
        const before = structure.columns.find((c) => c.name === one.from);
        columns.push({
          renameColumn: {
            from: one.from,
            to: one.to,
            spec: {
              name: one.to,
              dataType: before?.dataType ?? one.dataType ?? "TEXT",
              nullable: before?.nullable ?? true,
              default: "",
              comment: "",
              primaryKey: false,
              unique: false,
            },
          },
        });
        break;
      }
      case "setType":
        columns.push({
          setType: { name: currentName(one.name), dataType: one.dataType },
        });
        break;
      case "setNullable":
        columns.push({
          setNullable: {
            name: currentName(one.name),
            nullable: one.nullable,
            // Carried, because two of the three engines restate the column to
            // change nullability and the restatement needs the type.
            dataType: one.dataType,
          },
        });
        break;
      case "setDefault":
        columns.push({
          setDefault: {
            name: currentName(one.name),
            default: one.default ?? "",
          },
        });
        break;
      case "dropIndex":
        indexes.push({ drop: { name: one.name } });
        break;
      case "addIndex":
        indexes.push({
          create: {
            name: one.name,
            columns: one.columns,
            unique: Boolean(one.unique),
          },
        });
        break;
      default:
        // An edit this build does not know about is dropped rather than sent:
        // showing SQL for something that will not run is worse than not
        // offering it.
        break;
    }
  }

  // Dropping a key the column already carries, or an index on a column being
  // dropped, is refused by all three engines; the review would show a script
  // that cannot run.
  for (const index of structure.indexes ?? []) {
    if (index.primary) continue;
    if (index.columns.some((column) => dropped.has(column)))
      indexes.push({ drop: { name: index.name } });
  }

  return { columns, keys, indexes };
}

/** A line describing one edit, for the staged list. */
export function describeEdit(one) {
  switch (one.kind) {
    case "addColumn":
      return `Add ${one.column.name} ${one.column.dataType}`;
    case "dropColumn":
      return `Drop ${one.name}`;
    case "renameColumn":
      return `Rename ${one.from} to ${one.to}`;
    case "setType":
      return `${one.name} becomes ${one.dataType}`;
    case "setNullable":
      return `${one.name} becomes ${one.nullable ? "nullable" : "not null"}`;
    case "setDefault":
      return one.default
        ? `${one.name} defaults to ${one.default}`
        : `Drop the default on ${one.name}`;
    case "dropIndex":
      return `Drop index ${one.name}`;
    case "addIndex":
      return `Add ${one.unique ? "unique index" : "index"} ${one.name} on ${one.columns.join(", ")}`;
    default:
      return one.kind;
  }
}

/**
 * Whether an edit has already been made, so the same change is not staged
 * twice. Comparing by kind and target rather than by value, because two
 * renames of the same column are one change and the second one is what counts.
 */
export function sameEdit(one, other) {
  if (one.kind !== other.kind) return false;
  const target = (edit) => edit.name ?? edit.from ?? edit.kind;
  return target(one) === target(other);
}

/** Replaces an edit of the same kind on the same column, or adds it. */
export function stageChange(edits, change) {
  const at = edits.findIndex((one) => sameEdit(one, change));
  if (at === -1) return [...edits, change];
  const next = [...edits];
  next[at] = change;
  return next;
}

export const newEdit = edit;

/**
 * What about a plan is likely to be refused, before it is run.
 *
 * A statement that fails halfway has already been shown, so the cost of
 * finding out is a round trip and a rollback — except on MySQL, which commits
 * each DDL statement on its own, where it is a half-applied table.
 */
export function planWarnings(structure, plan) {
  const warnings = [];

  for (const change of plan.columns ?? []) {
    const [kind, body] = Object.entries(change)[0] ?? [];
    if (!body) continue;
    const column = structure.columns.find((one) => one.name === body.name);

    if (kind === "addColumn") {
      // A new NOT NULL column with no default has nothing to put in the rows
      // already there, and MySQL refuses exactly this on a populated table.
      if (body.nullable === false && !body.default)
        warnings.push(
          `${body.name} is added as not null with no default. If ${structure.name ?? "the table"} already has rows, the server will refuse this — give it a default, or add it as nullable.`,
        );
    } else if (
      kind === "setType" &&
      column &&
      column.nullable === false &&
      /int/i.test(column.dataType) &&
      /char|text/i.test(body.dataType)
    ) {
      // The column holds whole numbers and is becoming text: anything that no
      // longer fits will fail the change rather than be truncated quietly.
      warnings.push(
        `${body.name} holds ${column.dataType} and is becoming ${body.dataType}. Values that do not fit will fail the change.`,
      );
    }
  }
  return warnings;
}

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

  for (const one of edits) {
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
          setType: { name: one.name, dataType: one.dataType },
        });
        break;
      case "setNullable":
        columns.push({
          setNullable: {
            name: one.name,
            nullable: one.nullable,
            // Carried, because two of the three engines restate the column to
            // change nullability and the restatement needs the type.
            dataType: one.dataType,
          },
        });
        break;
      case "setDefault":
        columns.push({
          setDefault: { name: one.name, default: one.default ?? "" },
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

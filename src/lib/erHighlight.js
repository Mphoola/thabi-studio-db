/**
 * Which parts of a diagram to dim while one table is pointed at.
 *
 * Pure, and out of the component, because this is the logic that has broken
 * three times in a row: once when the helpers were never declared, and once
 * because it was written as one function taking a table *or* a line, so the
 * two callers passed arguments in different shapes and one of them passed
 * null where the other read a property off it.
 *
 * Two functions, one shape each.
 */

/** Does this line join the named table? A line knows its own two tables. */
export function lineTouches(line, name) {
  if (!line || !name) return false;
  return line.fromTable === name || line.toTable === name;
}

/** Should this line stand back? */
export function lineIsDimmed(line, name) {
  if (!name) return false;
  return !lineTouches(line, name);
}

/** Should this table box stand back? A table is either the one or it is not. */
export function tableIsDimmed(tableName, name) {
  if (!name) return false;
  return tableName !== name;
}

/** Is this line one of the named table's keys? Used to thicken and recolour. */
export function lineIsLit(line, name) {
  return lineTouches(line, name);
}

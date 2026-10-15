/**
 * Which columns a grid shows, and in what order.
 *
 * Everything here is about *display positions*: `order` is a list of source
 * column indexes in the order they appear, and `hidden` is the set of source
 * indexes the user turned off. The grid renders `visible` and maps a display
 * position back to a source index with it, so selection, editing and copy all
 * keep addressing the real column.
 */

/** The layout a fresh table starts with: every column, in its own order. */
export function defaultLayout() {
  return { order: [], hidden: [] };
}

/**
 * The source indexes to render, left to right.
 *
 * A stored order that is stale — a column dropped from the table, a new one
 * added — is healed rather than trusted: unknown indexes are dropped, and
 * columns missing from the order are appended in their own order, so a schema
 * change never hides data the user did not hide.
 */
export function visibleColumns(columns, layout) {
  const total = columns.length;
  const inRange = (index) =>
    Number.isInteger(index) && index >= 0 && index < total;
  const seen = new Set();
  const order = [];
  for (const index of layout?.order ?? []) {
    if (inRange(index) && !seen.has(index)) {
      seen.add(index);
      order.push(index);
    }
  }
  for (let index = 0; index < total; index += 1) {
    if (!seen.has(index)) order.push(index);
  }
  const hidden = new Set(
    (layout?.hidden ?? []).filter((index) => inRange(index)),
  );
  return order.filter((index) => !hidden.has(index));
}

/** A layout with `name`'s column hidden or shown. */
export function toggleHidden(columns, layout, name) {
  const index = columns.findIndex((column) => column.name === name);
  if (index < 0) return layout;
  const hidden = new Set(layout?.hidden ?? []);
  if (hidden.has(index)) hidden.delete(index);
  else hidden.add(index);
  return { order: layout?.order ?? [], hidden: [...hidden] };
}

/**
 * A layout with `name`'s column moved `delta` places along the display order,
 * swapping it with its neighbour. Hidden columns stay put in the stored order.
 */
export function moveColumn(columns, layout, name, delta) {
  const index = columns.findIndex((column) => column.name === name);
  if (index < 0) return layout;
  const current = visibleColumns(columns, layout);
  const at = current.indexOf(index);
  const to = at + delta;
  if (at < 0 || to < 0 || to >= current.length) return layout;
  const nextVisible = [...current];
  [nextVisible[at], nextVisible[to]] = [nextVisible[to], nextVisible[at]];

  // Write the new sequence back into the slots the visible columns occupy,
  // so a hidden column keeps its own place instead of being shuffled along.
  const total = columns.length;
  const hidden = new Set(
    (layout?.hidden ?? []).filter((i) => i >= 0 && i < total),
  );
  // An empty stored order means "never moved"; start from the healed order.
  const stored = layout?.order?.length ? layout.order : current;
  const merged = [...stored];
  let slot = 0;
  for (let i = 0; i < merged.length; i += 1) {
    const here = merged[i];
    if (here >= 0 && here < total && !hidden.has(here)) {
      merged[i] = nextVisible[slot];
      slot += 1;
    }
  }
  return { order: merged, hidden: [...hidden] };
}

/** Every column shown again, in its own order. */
export function resetLayout() {
  return defaultLayout();
}

/** A width that fits the widest value in the loaded rows. */
export function fitWidth(cells, index, typeName, { min = 60, max = 400 } = {}) {
  const natural = cells.reduce((widest, row) => {
    const cell = row?.[index];
    const length = typeof cell === "string" ? cell.length : 0;
    return Math.max(widest, length);
  }, 0);
  // Room for the name, its type, the padding and the sort indicator.
  const wanted = Math.max(natural * 7.6 + 56, 90);
  return Math.round(Math.min(max, Math.max(min, wanted)));
}

/**
 * How many rows a table view holds at once, and where the user is within a
 * table.
 *
 * A page is the chunk the user moves between with the page buttons, so its size
 * decides how far one request goes. It comes from the connection: a connection
 * set to 10,000 rows means 10,000 at a time, because that is the same answer
 * the connection already gives for a query result and the user should not have
 * to set it twice. A connection that sets nothing gets 1,000 — enough to be
 * useful, few enough that a table with a million rows opens at once.
 *
 * The ceiling matches the largest page the server will serve. A connection may
 * ask for more rows than are worth holding together, and a page that large is
 * the one case where paging stops helping.
 */

/** Rows a table view shows at once when the connection sets no limit. */
export const DEFAULT_PAGE_SIZE = 1_000;

/**
 * What a source reports when it could not estimate the size.
 *
 * A stand-in rather than null, because the grid needs a row count to size its
 * scroller. It is finite, so anything reading the total has to ask this instead
 * of `Number.isFinite`, or a table of no known size is labelled with nine
 * quadrillion rows.
 */
export const UNKNOWN_TOTAL = Number.MAX_SAFE_INTEGER;

/** Whether a total is a real number of rows. */
export function knownTotal(total) {
  return Number.isFinite(total) && total < UNKNOWN_TOTAL;
}

/** The most rows one page may hold; what the server will serve at most. */
export const MAX_PAGE_SIZE = 100_000;

/**
 * Rows per page for a table view.
 *
 * @param {{ maxResultRows?: number|string|null }} profile connection profile
 * @returns {number} at least 1, never more than `MAX_PAGE_SIZE`
 */
export function tablePageSize(profile) {
  const asked = Number(profile?.maxResultRows);
  if (!Number.isFinite(asked) || asked <= 0) return DEFAULT_PAGE_SIZE;
  // Rounded, because a fraction of a row is not a page, and floored at one row.
  return Math.min(MAX_PAGE_SIZE, Math.max(1, Math.round(asked)));
}

/** Pages a table of `total` rows takes, or null while the size is unknown. */
export function pageCount(total, pageSize) {
  if (!knownTotal(total) || total < 0) return null;
  return Math.max(1, Math.ceil(total / pageSize));
}

/**
 * The page a display position falls on, zero-based.
 *
 * The page number in the UI is one-based, but every offset in the paging maths
 * is zero-based, so the conversion lives here rather than at each call site.
 */
export function pageOf(position, pageSize) {
  return Math.floor(position / pageSize);
}

/** "Rows 1–1,000 of about 1,100,000", for the pager. */
export function describeRows(page, pageSize, total, estimated) {
  const first = page * pageSize + 1;
  const last = (page + 1) * pageSize;
  if (!knownTotal(total)) return `Rows ${first.toLocaleString()}–`;
  const about = estimated ? "about " : "";
  const end = Math.min(last, total);
  return `Rows ${first.toLocaleString()}–${end.toLocaleString()} of ${about}${total.toLocaleString()}`;
}

/**
 * What the pager needs, from the grid's paging state and the source.
 *
 * The grid is not there yet when a table is first rendered, so every value has
 * an answer that is not an error. Resolving them here rather than in a template
 * is what lets that be tested: this boundary has been read wrong twice, once
 * for treating a number as a call and once for the reverse, and neither was
 * caught by lint, the build or any test.
 *
 * @param {object|null} paging the grid's paging state, or null before it mounts
 * @param {object} source the grid's source: `total`, `estimated`, `pageSize`
 * @param {number} fallbackPageSize used while the grid has not answered
 */
export function pagerProps(paging, source, fallbackPageSize) {
  return {
    page: paging?.page ?? 0,
    pageSize: paging?.pageSize ?? source?.pageSize ?? fallbackPageSize,
    total: source?.total ?? null,
    estimated: Boolean(source?.estimated),
    // A short page is the end of the table; until one is seen, assume more.
    hasMore: paging?.hasMore ?? true,
  };
}

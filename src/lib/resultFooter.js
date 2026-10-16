/**
 * What the results bar says under a grid.
 *
 * A query result and a table page both end up in the same grid, and they mean
 * different things by the same numbers. For a query result `total` is how many
 * rows were fetched, and being cut short at the limit is worth warning about —
 * add a WHERE. For a table page `total` is how many rows the *table* has, and
 * being cut short is the whole point: the pager is there to fetch the rest.
 *
 * Reading one with the other's rules produced a bar reading "First 1,109,100
 * rows fetched — add a WHERE or LIMIT" directly above a pager reading "Rows
 * 1–1,000 of about 1,109,100". So the two are told apart by the source, and a
 * table's bar says only what it can stand behind.
 */

/**
 * @param {object} source the grid's source; `kind` is "table" or "result"
 * @param {object} result what the last read returned
 * @returns {{ summary: string, elapsedMs: string, warnings: string[] }}
 */
export function resultFooter(source, result = {}) {
  const elapsedMs = Number.isFinite(result.elapsedMs)
    ? `${result.elapsedMs.toLocaleString()} ms`
    : "";
  if (source?.kind === "table") {
    // The pager above already says how many rows are on screen and how many the
    // table has. Repeating the table's size here would only contradict it.
    return { summary: "", elapsedMs, warnings: [] };
  }
  const total = Number(source?.total ?? 0);
  const rows = total === 1 ? "1 row" : `${total.toLocaleString()} rows`;
  const warnings = [];
  if (result.truncated)
    warnings.push(
      "More rows exist. Add a WHERE or LIMIT, or raise the limit in Settings.",
    );
  if (result.extraResultSets)
    warnings.push(`${result.extraResultSets} more result sets not shown`);
  return {
    summary: result.truncated ? `First ${rows} fetched` : rows,
    elapsedMs,
    warnings,
  };
}

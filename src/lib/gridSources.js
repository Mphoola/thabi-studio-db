import {
  copyRows,
  countTableRows,
  exportCsv,
  fetchRows,
  fetchTablePage,
} from "./tauri/commands.js";
import { pickCsvDestination } from "./tauri/files.js";

/**
 * A grid source for a query result kept in Rust: pages, sorting, copying
 * and CSV export all happen there.
 */
export function remoteSource({ profileId, tabId, result, exportName }) {
  const id = result.resultId;
  return {
    columns: result.columns,
    total: result.totalRows ?? result.rows.length,
    firstRows: result.rows,
    sortable: Boolean(id),
    load: (offset, limit, sort) =>
      fetchRows(profileId, tabId, id, { offset, limit, sort }),
    copy: (scope, format, sort) =>
      copyRows(profileId, tabId, id, { scope, sort, format }),
    async export(scope, sort) {
      const path = await pickCsvDestination(exportName || "results");
      if (!path) return null;
      return exportCsv(profileId, tabId, id, { path, scope, sort });
    },
  };
}

/**
 * A grid source for a table tab: every page is read from the server with
 * LIMIT/OFFSET, and sorting re-queries rather than reordering the rows that
 * happen to be loaded (Phase 12).
 *
 * `estimate` is the catalog's guess at the table's size and is labelled as an
 * estimate wherever it is shown; `count()` asks for the exact number, which
 * costs a full scan on a large table, so it is never done unasked.
 */
export function tableSource({
  profileId,
  tabId,
  target,
  filters,
  primaryKey,
  estimate,
  columns = [],
  firstRows = [],
}) {
  return {
    columns,
    /** The grid shows "of about N" while this is all that is known. */
    total: estimate ?? Number.MAX_SAFE_INTEGER,
    estimated: true,
    firstRows,
    sortable: primaryKey?.length > 0,
    load: (offset, limit, sort) =>
      fetchTablePage(
        profileId,
        tabId,
        {
          ...target,
          filters,
          primaryKey: primaryKey ?? [],
          order: sort
            ? { column: sort.column, descending: sort.descending }
            : null,
        },
        { offset, limit },
      ),
    // Copy and export re-read the whole table, which the next increment adds;
    // until then they are absent rather than quietly covering one page.
    count: () => countTableRows(profileId, tabId, { ...target, filters }),
  };
}

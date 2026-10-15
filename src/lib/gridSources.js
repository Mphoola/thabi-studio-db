import { copyRows, exportCsv, fetchRows } from "./tauri/commands.js";
import { pickCsvDestination } from "./tauri/files.js";
import { rowsToText } from "./gridText.js";

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

/** A grid source for rows already in the page (table previews). */
export function localSource(result) {
  return {
    columns: result.columns,
    total: result.rows.length,
    firstRows: result.rows,
    sortable: false,
    load: async (offset, limit) => result.rows.slice(offset, offset + limit),
    copy: async (scope, format) => {
      const [start, end] = scope.rows ?? [0, result.rows.length];
      const cols = scope.columns ?? result.columns.map((_, i) => i);
      const rows = result.rows
        .slice(start, end)
        .map((row) => cols.map((c) => row[c]));
      return rowsToText(
        cols.map((c) => result.columns[c]),
        rows,
        { format, includeHeader: scope.includeHeader },
      );
    },
  };
}

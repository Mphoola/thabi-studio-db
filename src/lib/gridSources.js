import { copyRows, exportCsv, fetchRows } from "./tauri/commands.js";
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

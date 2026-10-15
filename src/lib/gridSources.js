import { isRef, shallowRef } from "vue";

import {
  copyRows,
  copyTable,
  countTableRows,
  exportCsv,
  exportTable,
  fetchRows,
  fetchTablePage,
} from "./tauri/commands.js";
import {
  EXPORT_FORMATS,
  pickExportDestination,
  pickCsvDestination,
} from "./tauri/files.js";

/**
 * A grid source for a query result kept in Rust: pages, sorting, copying
 * and CSV export all happen there.
 */
export function remoteSource({ profileId, tabId, result, exportName }) {
  const id = result.resultId;
  return {
    kind: "result",
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
  maxRows,
  pageSize,
}) {
  // One owner for the result's columns. The explorer owns the live list so it
  // can clear it while loading; a snapshot here would let the grid render
  // indexes the editing side no longer has, and every cell read would fail.
  const columnsRef = isRef(columns) ? columns : shallowRef(columns);
  return {
    get columns() {
      return columnsRef.value;
    },
    /** The grid shows "of about N" while this is all that is known. */
    kind: "table",
    total: estimate ?? Number.MAX_SAFE_INTEGER,
    estimated: true,
    firstRows,
    sortable: primaryKey?.length > 0,
    // The chunk the user moves between with the page buttons.
    pageSize,
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
    // Copy and export re-read the table from the server, so they cover the
    // whole table and not just the page on screen.
    copy: (scope, format, sort) =>
      copyTable(
        profileId,
        tabId,
        {
          ...target,
          filters,
          primaryKey: primaryKey ?? [],
          columns: columns.map((column) => column.name),
          order: sort
            ? { column: sort.column, descending: sort.descending }
            : null,
        },
        { format, maxRows: maxRows ?? null, scope: toScope(scope) },
      ),
    async export(scope, sort, format) {
      const chosen =
        EXPORT_FORMATS.find((f) => f.value === format) ?? EXPORT_FORMATS[0];
      const path = await pickExportDestination(
        target?.name || "table",
        chosen.extension,
      );
      if (!path) return null;
      return exportTable(
        profileId,
        tabId,
        {
          ...target,
          filters,
          primaryKey: primaryKey ?? [],
          columns: columns.map((column) => column.name),
          order: sort
            ? { column: sort.column, descending: sort.descending }
            : null,
        },
        { format: chosen.value, path, scope: toScope(scope) },
      );
    },
    count: () => countTableRows(profileId, tabId, { ...target, filters }),
  };
}

/** The grid's copy scope in the shape the transfer commands take. */
function toScope(scope = {}) {
  return {
    columns: scope.columns ?? [],
    rows: scope.rows ?? null,
    includeHeader: scope.includeHeader ?? true,
  };
}

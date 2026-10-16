import { copyText } from "../../lib/clipboard.js";
import { computed } from "vue";
import {
  ArrowUpRight,
  ClipboardCopy,
  Download,
  Filter,
  FilterX,
  Pencil,
  RotateCcw,
  Search,
  Trash2,
} from "lucide-vue-next";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail } from "../../lib/errors.js";

/** Copy and export for the grid's selection, through the data source. */
export function useGridActions(props, data, selection) {
  const toasts = useToasts();

  function scope(includeHeader, wholeRows = false) {
    const r = selection.range.value;
    if (!selection.active.value) return { includeHeader };
    return {
      rows: [r.r0, r.r1 + 1],
      // The selection is in display positions; copy and export address
      // source columns, so the choice is mapped here.
      columns: wholeRows
        ? null
        : Array.from({ length: r.c1 - r.c0 + 1 }, (_, i) => r.c0 + i).map(
            toSourceColumn,
          ),
      includeHeader,
    };
  }

  function toSourceColumn(position) {
    return props.columnIndexes?.[position] ?? position;
  }

  async function copy(format, includeHeader = false, wholeRows = false) {
    const s = scope(includeHeader, wholeRows);
    try {
      const text = await props.source.copy(s, format, data.sort.value);
      await copyText(text);
      const count = s.rows ? s.rows[1] - s.rows[0] : props.source.total;
      toasts.info(
        `Copied ${count.toLocaleString()} ${count === 1 ? "row" : "rows"}`,
      );
    } catch (error) {
      toasts.error("Couldn’t copy", toUserDetail(error));
    }
  }

  /** `format` comes from the export menu; null means the source's default. */
  async function exportAll(format = null) {
    try {
      const summary = await props.source.export?.(
        { includeHeader: true },
        data.sort.value,
        format,
      );
      if (summary)
        toasts.success(
          `Exported ${summary.rows.toLocaleString()} rows`,
          summary.path,
        );
    } catch (error) {
      toasts.error("Couldn’t export", toUserDetail(error));
    }
  }

  const menuItems = computed(() => [
    // A source that has nothing to copy from (a table read a page at a time,
    // until export re-reads it) simply doesn't offer copying.
    ...(props.source.copy
      ? [
          { id: "copy", label: "Copy", icon: ClipboardCopy, shortcut: "Mod+C" },
          {
            id: "copy-header",
            label: "Copy with Headers",
            shortcut: "Mod+Shift+C",
          },
          { id: "copy-csv", label: "Copy as CSV" },
          { id: "copy-json", label: "Copy as JSON" },
          { id: "copy-markdown", label: "Copy as Markdown" },
          { id: "copy-insert", label: "Copy as SQL INSERT" },
          { id: "copy-update", label: "Copy as SQL UPDATE" },
          { id: "copy-rows", label: "Copy Rows" },
          { separator: true },
        ]
      : []),
    { id: "inspect", label: "Inspect Value", icon: Search },
    ...(props.quickFilter
      ? [
          { separator: true },
          { id: "filter-value", label: "Filter by This Value", icon: Filter },
          { id: "filter-exclude", label: "Exclude This Value", icon: FilterX },
        ]
      : []),
    ...links().map((link, index) => ({
      id: `link-${index}`,
      label: link.label,
      icon: ArrowUpRight,
    })),
    ...(props.editor?.enabled.value
      ? [
          { separator: true },
          { id: "edit-cell", label: "Edit Cell", icon: Pencil },
          { id: "set-null", label: "Set to NULL" },
          { id: "set-default", label: "Set to DEFAULT" },
          { id: "duplicate-rows", label: "Duplicate Rows", shortcut: "Mod+D" },
          { id: "paste-rows", label: "Paste as New Rows", shortcut: "Mod+V" },
          { id: "delete-rows", label: "Delete Rows", icon: Trash2 },
          { id: "revert-rows", label: "Revert Changes", icon: RotateCcw },
        ]
      : []),
    ...(props.source.export
      ? [
          { separator: true },
          { id: "export", label: "Export CSV…", icon: Download },
        ]
      : []),
  ]);

  /** The focused cell and its source column's name. */
  function focused() {
    const { row, col } = selection.focus.value;
    const cells = data.rowAt(row);
    const source = toSourceColumn(col);
    const column = props.source.columns[source];
    return cells && column ? { name: column.name, cell: cells[source] } : null;
  }

  function links() {
    const here = props.cellLinks && selection.active.value ? focused() : null;
    return here ? props.cellLinks(here.name, here.cell) : [];
  }

  function run(id) {
    if (id === "filter-value" || id === "filter-exclude") {
      const here = focused();
      if (here)
        props.quickFilter(here.name, here.cell, id === "filter-exclude");
      return;
    }
    if (id.startsWith("link-")) return links()[Number(id.slice(5))]?.run();
    if (id === "copy") return copy("tsv");
    if (id === "copy-header") return copy("tsv", true);
    if (id === "copy-csv") return copy("csv", true);
    // Structured formats always take whole columns' names, so headers on.
    if (id === "copy-json") return copy("json", true);
    if (id === "copy-markdown") return copy("markdown", true);
    if (id === "copy-insert") return copy("sqlInsert", true);
    // UPDATE needs the key columns, so it copies whole rows.
    if (id === "copy-update") return copy("sqlUpdate", true, true);
    if (id === "copy-rows") return copy("tsv", false, true);
    // The toolbar's Export… menu offers every format; here the default (CSV).
    if (id === "export") return exportAll();
  }

  return { copy, exportAll, menuItems, run };
}

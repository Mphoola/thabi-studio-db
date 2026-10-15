import { copyText } from "../../lib/clipboard.js";
import { computed } from "vue";
import {
  ClipboardCopy,
  Download,
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
          { id: "copy-rows", label: "Copy Rows" },
          { separator: true },
        ]
      : []),
    { id: "inspect", label: "Inspect Value", icon: Search },
    ...(props.editor?.enabled.value
      ? [
          { separator: true },
          { id: "edit-cell", label: "Edit Cell", icon: Pencil },
          { id: "set-null", label: "Set to NULL" },
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

  function run(id) {
    if (id === "copy") return copy("tsv");
    if (id === "copy-header") return copy("tsv", true);
    if (id === "copy-csv") return copy("csv", true);
    if (id === "copy-rows") return copy("tsv", false, true);
    // The toolbar's Export… menu offers every format; here the default (CSV).
    if (id === "export") return exportAll();
  }

  return { copy, exportAll, menuItems, run };
}

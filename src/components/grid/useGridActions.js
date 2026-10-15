import { copyText } from "../../lib/clipboard.js";
import { computed } from "vue";
import { ClipboardCopy, Download, Search } from "lucide-vue-next";
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
      columns: wholeRows
        ? null
        : Array.from({ length: r.c1 - r.c0 + 1 }, (_, i) => r.c0 + i),
      includeHeader,
    };
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

  async function exportAll() {
    try {
      const summary = await props.source.export?.(
        { includeHeader: true },
        data.sort.value,
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
    { id: "copy", label: "Copy", icon: ClipboardCopy, shortcut: "Mod+C" },
    { id: "copy-header", label: "Copy with Headers", shortcut: "Mod+Shift+C" },
    { id: "copy-csv", label: "Copy as CSV" },
    { id: "copy-rows", label: "Copy Rows" },
    { separator: true },
    { id: "inspect", label: "Inspect Value", icon: Search },
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
    if (id === "export") return exportAll();
  }

  return { copy, exportAll, menuItems, run };
}

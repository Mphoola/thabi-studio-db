import { computed, ref, watch } from "vue";
import { isNumericType } from "../../lib/gridText.js";
import { fitWidth } from "../../lib/columns.js";

/** Column widths for the results grid: defaults by type, fit-to-contents. */
export function useColumnWidths({
  columns,
  columnIndexes,
  data,
  numberColumn,
}) {
  const widths = ref([]);
  /** Fit-to-contents widths, applied until the user resizes a column. */
  const fitted = ref(null);
  watch(
    columns,
    (cols) => {
      if (fitted.value?.length === cols.length) {
        widths.value = [...fitted.value];
        return;
      }
      widths.value = cols.map((c) => (isNumericType(c.typeName) ? 120 : 180));
    },
    { immediate: true },
  );

  /** Widths that fit the rows loaded so far. */
  function fitToContents() {
    const rows = data.loadedRows();
    fitted.value = columns.value.map((column, position) =>
      fitWidth(
        rows.map((row) => row?.[columnIndexes.value[position]]?.value ?? ""),
        0,
        column.typeName,
      ),
    );
    widths.value = [...fitted.value];
  }
  const template = computed(
    () => `${numberColumn}px ${widths.value.map((w) => `${w}px`).join(" ")}`,
  );

  function resizeColumn(index, width) {
    // A manual resize wins over "fit to contents" until the columns change.
    fitted.value = null;
    widths.value[index] = Math.min(800, Math.max(60, width));
  }

  return { widths, fitToContents, resizeColumn, template };
}

import { computed, ref, watch } from "vue";
import {
  defaultLayout,
  moveColumn,
  resetLayout,
  toggleHidden,
  visibleColumns,
} from "../lib/columns.js";

/**
 * Which columns a table tab shows, remembered on the tab so it survives
 * switching tabs and coming back.
 *
 * The layout is stored as source column indexes, and healed when the table
 * changes shape, so a renamed or dropped column can't leave a broken grid.
 */
export function useColumns({ columns, layout }) {
  // A ref that mirrors the tab's stored layout, so it can be replaced wholesale.
  const stored = ref(layout?.value ?? defaultLayout());
  watch(layout, (next) => (stored.value = next ?? defaultLayout()), {
    immediate: true,
  });

  /** Source column indexes to render, left to right. */
  const visible = computed(() => visibleColumns(columns.value, stored.value));
  const visibleColumnsMeta = computed(() =>
    visible.value.map((index) => columns.value[index]),
  );
  const hiddenNames = computed(() => {
    const shown = new Set(visible.value);
    return columns.value
      .filter((_, index) => !shown.has(index))
      .map((column) => column.name);
  });

  function save(next) {
    stored.value = next;
    if (layout) layout.value = next;
  }

  function toggle(name) {
    save(toggleHidden(columns.value, stored.value, name));
  }

  function move(name, delta) {
    const next = moveColumn(columns.value, stored.value, name, delta);
    if (next !== stored.value) save(next);
  }

  function reset() {
    save(resetLayout());
  }

  const isHidden = (name) => hiddenNames.value.includes(name);

  return {
    visible,
    visibleColumns: visibleColumnsMeta,
    hiddenNames,
    isHidden,
    toggle,
    move,
    reset,
  };
}

/** A ref holding a per-tab column layout, created on first use. */
export function columnLayoutRef(tab) {
  if (!tab.columnLayout) {
    tab.columnLayout = { order: [], hidden: [] };
  }
  return computed({
    get: () => tab.columnLayout,
    set: (value) => (tab.columnLayout = value),
  });
}

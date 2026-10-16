import { computed, ref } from "vue";
import { useFilterPresets } from "../../composables/useFilterPresets.js";
import { newFilter, toRequest } from "../../lib/rowFilters.js";
import { previewFilter } from "../../lib/tauri/commands.js";

/**
 * The filter bar's state for a table tab. Filters live on the tab so they
 * survive switching tabs; presets live with the table (useFilterPresets).
 */
export function useTableFilters({
  tab,
  node,
  profileId,
  columns,
  view,
  reload,
  toasts,
}) {
  if (!tab.filters) tab.filters = [];
  const filters = computed({
    get: () => tab.filters,
    set: (value) => (tab.filters = value),
  });
  const show = computed(() => view.value === "data" && tab.showFilters);
  // How many filters the rows on screen were read with.
  const appliedCount = ref(0);

  function toggle() {
    tab.showFilters = !tab.showFilters;
    if (tab.showFilters && !filters.value.length)
      filters.value = [newFilter(columns.value[0]?.name ?? "")];
  }

  /** "Filter by This Value" / "Exclude This Value" from a cell. */
  function quickFilter(column, cell, exclude) {
    const isNull = cell?.kind === "null";
    const filter = {
      ...newFilter(column),
      op: isNull ? (exclude ? "not_null" : "is_null") : exclude ? "ne" : "eq",
      value: isNull ? "" : (cell?.value ?? ""),
    };
    filters.value = [...filters.value.filter((f) => f.column), filter];
    tab.showFilters = true;
    reload();
  }

  const presets = useFilterPresets({
    profileId,
    target: node,
    filters,
    toasts,
  });
  const presetMenu = computed(() => ({
    list: presets.presets.value,
    loading: presets.loading.value,
    currentName: presets.existingName(),
    apply: presets.apply,
    save: presets.save,
    remove: presets.remove,
  }));

  /** The WHERE clause the ticked filters make, built by Rust as it runs it. */
  function previewSql() {
    return previewFilter(profileId.value, tab.id, toRequest(filters.value));
  }

  return {
    filters,
    previewSql,
    show,
    appliedCount,
    toggle,
    quickFilter,
    presetMenu,
    request: () => toRequest(filters.value),
  };
}

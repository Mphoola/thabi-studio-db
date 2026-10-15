import { ref, watch } from "vue";
import { toUserMessage } from "../lib/errors.js";
import {
  deleteFilterPreset,
  listFilterPresets,
  saveFilterPreset,
} from "../lib/tauri/commands.js";

/**
 * The filter presets saved for one table.
 *
 * A preset belongs to a table on a connection rather than to a tab, so it is
 * here after the tab is closed and after a restart. Applying one hands the
 * filter bar back the filters it holds, which is why they are stored as the bar
 * wrote them and not translated.
 *
 * The list is read whenever the table changes, so a preset saved on one tab is
 * offered on the next one opened on the same table.
 */
export function useFilterPresets({ profileId, target, filters, toasts }) {
  const presets = ref([]);
  const loading = ref(false);

  /** Where a preset is saved: one connection, one table. */
  function scope() {
    const node = target?.value;
    return {
      profileId: profileId?.value ?? "",
      database: node?.database ?? "",
      schema: node?.schema ?? "",
      name: node?.name ?? "",
    };
  }

  /** The filters a preset is made of: the ones switched on. */
  function enabled() {
    return filters.value
      .filter((filter) => filter.enabled)
      .map((filter) => ({
        column: filter.column,
        op: filter.op,
        value: filter.value,
      }));
  }

  function ready() {
    const { profileId: id, name } = scope();
    return Boolean(id && name);
  }

  async function refresh() {
    if (!ready()) {
      presets.value = [];
      return;
    }
    const { profileId: id, database, schema, name } = scope();
    loading.value = true;
    try {
      presets.value = await listFilterPresets(id, database, schema, name);
    } catch (error) {
      presets.value = [];
      toasts?.error("Couldn't read saved filters", toUserMessage(error));
    } finally {
      loading.value = false;
    }
  }

  /** Saves the filters as they stand; a name already used is that one again. */
  async function save(name) {
    const trimmed = String(name ?? "").trim();
    if (!trimmed) return null;
    if (!enabled().some((filter) => filter.value !== "")) {
      toasts?.error("Nothing to save", "Add at least one filter first.");
      return null;
    }
    const { profileId: id, database, schema, name: object } = scope();
    try {
      const saved = await saveFilterPreset({
        profileId: id,
        databaseName: database,
        schemaName: schema,
        objectName: object,
        name: trimmed,
        filters: enabled(),
      });
      await refresh();
      toasts?.success(`Saved “${saved.name}”`);
      return saved;
    } catch (error) {
      toasts?.error("Couldn't save these filters", toUserMessage(error));
      return null;
    }
  }

  /** Puts a preset's filters back on the table. */
  function apply(preset) {
    if (!preset) return;
    filters.value = preset.filters.map((filter) => ({
      ...filter,
      enabled: filter.enabled !== false,
    }));
  }

  async function remove(preset) {
    try {
      await deleteFilterPreset(preset.id);
      presets.value = presets.value.filter((one) => one.id !== preset.id);
      toasts?.success(`Deleted “${preset.name}”`);
    } catch (error) {
      toasts?.error("Couldn't delete that filter preset", toUserMessage(error));
    }
  }

  /** The name already saved here, if this filter set is one of them. */
  function existingName() {
    const wanted = JSON.stringify(enabled());
    const found = presets.value.find(
      (preset) => JSON.stringify(preset.filters) === wanted,
    );
    return found?.name ?? "";
  }

  watch(
    () => [scope().profileId, scope().database, scope().schema, scope().name],
    refresh,
    { immediate: true },
  );

  return { presets, loading, refresh, save, apply, remove, existingName };
}

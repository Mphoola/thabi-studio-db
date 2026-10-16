<script setup>
import { isModKey } from "../../lib/platform.js";
import { computed, ref, watch } from "vue";
import { formatShortcut } from "../../lib/shortcuts.js";
import { toUserMessage } from "../../lib/errors.js";
import { Minus, Plus } from "lucide-vue-next";
import {
  FILTER_OPERATORS,
  RAW_SQL,
  needsValue,
  newFilter,
} from "../../lib/rowFilters.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";
import SelectInput from "../common/SelectInput.vue";
import TextInput from "../common/TextInput.vue";
import FilterPresets from "./FilterPresets.vue";

/** Filter rows above table data: column, operator, value; or raw SQL. */
const filters = defineModel("filters", { type: Array, required: true });
const props = defineProps({
  columns: { type: Array, required: true },
  /** How many filters the shown rows were read with. */
  appliedCount: { type: Number, default: 0 },
  loading: { type: Boolean, default: false },
  /** Saved filter sets for this table. */
  presets: { type: Object, default: null },
  /** () => Promise<string>: the WHERE clause, for Show SQL. */
  previewSql: { type: Function, default: null },
});
const emit = defineEmits(["apply"]);

const columnOptions = computed(() => [
  ...props.columns.map((name) => ({ value: name, label: name })),
  { value: RAW_SQL, label: "Raw SQL…" },
]);

function add(after) {
  const next = [...filters.value];
  next.splice(after + 1, 0, newFilter(props.columns[0] ?? ""));
  filters.value = next;
}

function remove(index) {
  filters.value = filters.value.filter((_, i) => i !== index);
  if (!filters.value.length) emit("apply");
}

// "Show SQL": the clause Rust will run, refreshed while it is open.
const sql = ref(null);
async function refreshSql() {
  if (sql.value === null || !props.previewSql) return;
  try {
    sql.value = (await props.previewSql()) || "No filters ticked.";
  } catch (error) {
    sql.value = toUserMessage(error);
  }
}
function toggleSql() {
  sql.value = sql.value === null ? "" : null;
  refreshSql();
}
watch(filters, refreshSql, { deep: true });

/** Mod+Return applies from anywhere in the bar, as Return does in a field. */
function onKeydown(event) {
  if (event.key === "Enter" && isModKey(event)) {
    event.preventDefault();
    emit("apply");
  }
}

function clear() {
  filters.value = [];
  emit("apply");
}
</script>

<template>
  <form
    class="flex shrink-0 flex-col gap-1.5 border-b border-line bg-toolbar px-4 py-2"
    aria-label="Row filters"
    @submit.prevent="emit('apply')"
    @keydown="onKeydown"
  >
    <div
      v-for="(filter, index) in filters"
      :key="index"
      class="flex items-center gap-1.5"
      :class="!filter.enabled && 'opacity-60'"
    >
      <input
        v-model="filter.enabled"
        type="checkbox"
        class="size-4 shrink-0 appearance-none rounded-sm border border-line-strong bg-surface checked:border-accent checked:bg-accent focus-visible:ring-2 focus-visible:ring-focus"
        :aria-label="filter.enabled ? 'Turn Filter Off' : 'Turn Filter On'"
        :title="
          filter.enabled
            ? 'On: unticked filters are kept but not applied'
            : 'Off'
        "
      />
      <SelectInput
        v-model="filter.column"
        class="w-48 shrink-0"
        :options="columnOptions"
        aria-label="Column"
      />
      <template v-if="filter.column === RAW_SQL">
        <TextInput
          v-model="filter.value"
          class="min-w-0 flex-1"
          placeholder="status = 'pending' AND total > 100"
          aria-label="SQL condition"
          mono
        />
      </template>
      <template v-else>
        <SelectInput
          v-model="filter.op"
          class="w-36 shrink-0"
          :options="FILTER_OPERATORS"
          aria-label="Operator"
        />
        <TextInput
          v-if="needsValue(filter.op)"
          v-model="filter.value"
          class="min-w-0 flex-1"
          :placeholder="filter.op.endsWith('in') ? 'a, b, c' : 'Value'"
          aria-label="Value"
          mono
        />
        <span v-else class="flex-1" />
      </template>
      <IconButton
        :icon="Minus"
        size="sm"
        label="Remove Filter"
        @click="remove(index)"
      />
      <IconButton
        :icon="Plus"
        size="sm"
        label="Add Filter"
        @click="add(index)"
      />
    </div>
    <div class="flex items-center gap-2">
      <span class="text-xs text-fg-subtle">
        {{
          appliedCount
            ? `${appliedCount} ${appliedCount === 1 ? "filter" : "filters"} applied. ${formatShortcut("Mod+Enter")} applies changes.`
            : `Return or ${formatShortcut("Mod+Enter")} applies the ticked filters.`
        }}
      </span>
      <BaseButton
        v-if="previewSql"
        size="sm"
        variant="ghost"
        class="ml-auto"
        :aria-pressed="sql !== null"
        @click="toggleSql"
      >
        {{ sql === null ? "Show SQL" : "Hide SQL" }}
      </BaseButton>
      <BaseButton size="sm" :class="!previewSql && 'ml-auto'" @click="clear">
        Clear
      </BaseButton>
      <FilterPresets
        v-if="presets"
        :presets="presets.list"
        :loading="presets.loading"
        :current-name="presets.currentName"
        :can-save="filters.some((f) => f.enabled)"
        @apply="presets.apply($event)"
        @save="presets.save($event)"
        @remove="presets.remove($event)"
      />
      <BaseButton
        v-tooltip="{ text: 'Apply All', shortcut: 'Mod+Enter' }"
        size="sm"
        variant="primary"
        type="submit"
        :loading="loading"
      >
        Apply All
      </BaseButton>
    </div>
    <p
      v-if="sql !== null"
      class="selectable truncate font-mono text-xs text-fg-muted"
      :title="sql"
    >
      {{ sql }}
    </p>
  </form>
</template>

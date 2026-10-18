<script setup>
import { computed } from "vue";
import { Eye, ListFilter, Plus, RefreshCw, Table2 } from "lucide-vue-next";
import IconButton from "../common/IconButton.vue";
import SegmentedControl from "../common/SegmentedControl.vue";

/** The table tab's title row: name, Data/Structure, and its actions. */
const view = defineModel("view", { type: String, required: true });
const props = defineProps({
  node: { type: Object, required: true },
  canAddRow: { type: Boolean, default: false },
  filtersShown: { type: Boolean, default: false },
  appliedCount: { type: Number, default: 0 },
  loading: { type: Boolean, default: false },
});
defineEmits(["add-row", "toggle-filters", "reload"]);

const views = [
  { value: "data", label: "Data" },
  { value: "structure", label: "Structure" },
];
const location = computed(() =>
  [props.node.database, props.node.schema].filter(Boolean).join(" · "),
);
</script>

<template>
  <header
    class="flex h-11 shrink-0 items-center gap-3 border-b border-line bg-toolbar px-4"
  >
    <component
      :is="node.kind === 'table' ? Table2 : Eye"
      :size="16"
      :stroke-width="1.75"
      class="text-fg-subtle"
      aria-hidden="true"
    />
    <div class="flex min-w-0 flex-col">
      <h1 class="truncate text-base font-semibold">{{ node.name }}</h1>
      <span class="truncate text-xs text-fg-subtle">{{ location }}</span>
    </div>
    <SegmentedControl
      v-model="view"
      class="ml-auto"
      :options="views"
      label="Object view"
    />
    <!-- Whatever the caller puts here sits before the buttons, so a table's
         actions end up beside the view switch. -->
    <slot />
    <IconButton
      v-if="view === 'data' && canAddRow"
      :icon="Plus"
      label="Add Row"
      @click="$emit('add-row')"
    />
    <IconButton
      v-if="view === 'data'"
      :icon="ListFilter"
      :label="appliedCount ? `Filters (${appliedCount})` : 'Filter Rows'"
      :pressed="filtersShown"
      @click="$emit('toggle-filters')"
    />
    <IconButton
      :icon="RefreshCw"
      label="Reload"
      :disabled="loading"
      @click="$emit('reload')"
    />
  </header>
</template>

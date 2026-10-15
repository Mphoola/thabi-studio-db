<script setup>
import { computed, ref, watch } from "vue";
import { Eye, ListFilter, Plus, RefreshCw, Table2 } from "lucide-vue-next";
import { useExplorer } from "../../composables/useExplorer.js";
import { describeObject, openObjectData } from "../../lib/tauri/commands.js";
import { useAppSettings } from "../../composables/useAppSettings.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";
import InlineAlert from "../common/InlineAlert.vue";
import SegmentedControl from "../common/SegmentedControl.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";
import ResultView from "../grid/ResultView.vue";
import { remoteSource } from "../../lib/gridSources.js";
import { newFilter, toRequest } from "../../lib/rowFilters.js";
import AddRowDialog from "./AddRowDialog.vue";
import ChangesPreviewDialog from "./ChangesPreviewDialog.vue";
import EditChangesBar from "./EditChangesBar.vue";
import FilterBar from "./FilterBar.vue";
import { useTableEditing } from "./useTableEditing.js";
import StructureView from "./StructureView.vue";

/** A table or view opened from the explorer: read-only data or structure. */
const props = defineProps({ tab: { type: Object, required: true } });
const explorer = useExplorer();
const { settings } = useAppSettings();
const node = computed(() => props.tab.node);
const tab = computed({
  get: () => props.tab.view ?? "data",
  set: (value) => (props.tab.view = value),
});
const tabs = [
  { value: "data", label: "Data" },
  { value: "structure", label: "Structure" },
];

const results = ref({}); // tab -> { status, value?, error? }
// Filters live on the tab so they survive switching tabs.
if (!props.tab.filters) props.tab.filters = [];
const filters = computed({
  get: () => props.tab.filters,
  set: (value) => (props.tab.filters = value),
});
const showFilters = computed(
  () => tab.value === "data" && props.tab.showFilters,
);
const appliedCount = ref(0);
// Columns of the last rows read, so the bar keeps working after an error.
const columns = ref([]);

function toggleFilters() {
  props.tab.showFilters = !props.tab.showFilters;
  if (props.tab.showFilters && !filters.value.length)
    filters.value = [newFilter(columns.value[0]?.name ?? "")];
}

const editing = useTableEditing({
  tab: props.tab,
  node,
  profileId: computed(() => props.tab.profileId ?? explorer.profile.value?.id),
  columns,
  reload: () => load("data"),
});
const editor = editing.editor;
const addingRow = ref(false);
// Save (Mod+S) in this tab applies the staged edits.
props.tab.applyEdits = editing.apply;

async function load(which = tab.value) {
  const target = node.value;
  const id = props.tab.profileId ?? explorer.profile.value?.id;
  if (!target || !id) return;
  results.value = { ...results.value, [which]: { status: "loading" } };
  const request = which === "data" ? toRequest(filters.value) : [];
  try {
    const value =
      which === "data"
        ? await openObjectData(id, props.tab.id, {
            ...target,
            maxRows: settings.value.maxResultRows || null,
            filters: request,
          })
        : await describeObject(id, target);
    if (node.value !== target) return;
    results.value = { ...results.value, [which]: { status: "loaded", value } };
    if (which === "data") {
      columns.value = value.columns;
      appliedCount.value = request.length;
    }
  } catch (error) {
    if (node.value === target)
      results.value = { ...results.value, [which]: { status: "error", error } };
  }
}

watch(node, () => {
  results.value = {};
  columns.value = [];
  load();
});
watch(tab, (value) => !results.value[value] && load(value));
if (node.value) load();

const current = computed(() => results.value[tab.value] ?? { status: "idle" });
const previewSource = computed(() =>
  tab.value === "data" && current.value.status === "loaded"
    ? remoteSource({
        profileId: props.tab.profileId,
        tabId: props.tab.id,
        result: current.value.value,
        exportName: node.value.name,
      })
    : null,
);
const location = computed(() =>
  [node.value?.database, node.value?.schema].filter(Boolean).join(" · "),
);
</script>

<template>
  <section v-if="node" class="flex h-full min-h-0 flex-col">
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
        v-model="tab"
        class="ml-auto"
        :options="tabs"
        label="Object view"
      />
      <IconButton
        v-if="tab === 'data' && editing.editable.value && columns.length"
        :icon="Plus"
        label="Add Row"
        @click="addingRow = true"
      />
      <IconButton
        v-if="tab === 'data'"
        :icon="ListFilter"
        :label="appliedCount ? `Filters (${appliedCount})` : 'Filter Rows'"
        :pressed="!!props.tab.showFilters"
        @click="toggleFilters"
      />
      <IconButton
        :icon="RefreshCw"
        label="Reload"
        :disabled="current.status === 'loading'"
        @click="load()"
      />
    </header>
    <FilterBar
      v-if="showFilters && columns.length"
      v-model:filters="filters"
      :columns="columns.map((c) => c.name)"
      :applied-count="appliedCount"
      :loading="current.status === 'loading'"
      @apply="load('data')"
    />
    <EditChangesBar
      v-if="tab === 'data' && editor.summary.value.total"
      :summary="editor.summary.value"
      :applying="editing.applying.value"
      @preview="editing.showPreview"
      @discard="editing.discard"
      @apply="editing.apply"
    />
    <div
      v-if="current.status === 'loading' || current.status === 'idle'"
      class="flex flex-1 items-center justify-center gap-2 text-fg-subtle"
      role="status"
    >
      <SpinnerIcon :size="14" />
      {{ tab === "data" ? "Reading rows…" : "Reading structure…" }}
    </div>
    <div v-else-if="current.status === 'error'" class="p-4">
      <InlineAlert
        tone="danger"
        :title="current.error?.message ?? 'Couldn’t read this object.'"
        :message="current.error?.detail ?? ''"
        mono
      >
        <BaseButton size="sm" @click="load()">Try Again</BaseButton>
      </InlineAlert>
    </div>
    <ResultView
      v-else-if="previewSource"
      class="min-h-0 flex-1"
      :source="previewSource"
      :result="current.value"
      :editor="editing.editable.value ? editor : null"
      :note="
        editing.readOnlyReason.value ||
        (editing.editable.value ? 'Double-click a cell to edit' : '')
      "
    />
    <StructureView
      v-else
      class="min-h-0 flex-1"
      :structure="current.value"
      :kind="node.kind"
    />
    <AddRowDialog
      v-model:open="addingRow"
      :columns="columns"
      :table="node.name"
      @add="editor.addRow"
    />
    <ChangesPreviewDialog
      :statements="editing.preview.value"
      :applying="editing.applying.value"
      @close="editing.preview.value = null"
      @apply="editing.apply"
    />
  </section>
</template>

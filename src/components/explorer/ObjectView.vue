<script setup>
import { computed, ref, watch } from "vue";
import { Eye, ListFilter, Plus, RefreshCw, Table2 } from "lucide-vue-next";
import { useExplorer } from "../../composables/useExplorer.js";
import { useFilterPresets } from "../../composables/useFilterPresets.js";
import { useToasts } from "../../composables/useToasts.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { describeObject, openObjectData } from "../../lib/tauri/commands.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";
import InlineAlert from "../common/InlineAlert.vue";
import SegmentedControl from "../common/SegmentedControl.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";
import ResultView from "../grid/ResultView.vue";
import TablePager from "../grid/TablePager.vue";
import { columnLayoutRef } from "../../composables/useColumns.js";
import { tableSource } from "../../lib/gridSources.js";
import { pagerProps, tablePageSize } from "../../lib/tablePaging.js";
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
const connections = useConnectionProfiles();
const toasts = useToasts();
const profile = computed(() => {
  const id = props.tab.profileId ?? explorer.profile.value?.id;
  return connections.profiles.value.find((p) => p.id === id) ?? null;
});
/** Rows this connection shows at once; see lib/tablePaging.js. */
const pageSize = computed(() => tablePageSize(profile.value));
const node = computed(() => props.tab.node);
// Saved filter sets belong to the table, so they are read whenever it changes.
const presetState = useFilterPresets({
  profileId: computed(() => props.tab.profileId ?? explorer.profile.value?.id),
  target: node,
  filters,
  toasts,
});
const gridView = ref(null);
/** The filter presets menu's view of the table's presets. */
const presetMenu = computed(() => ({
  list: presetState.presets.value,
  loading: presetState.loading.value,
  currentName: presetState.existingName(),
  apply: presetState.apply,
  save: presetState.save,
  remove: presetState.remove,
}));

/** What the pager shows; see lib/tablePaging.js. */
const pager = computed(() =>
  pagerProps(
    gridView.value?.paging ?? null,
    previewSource.value,
    pageSize.value,
  ),
);
const goToPage = (index) => gridView.value?.goToPage(index);
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

// Column layout lives on the tab, so a table remembers it next time.
const columnLayout = columnLayoutRef(props.tab);

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
            // One page at a time, ordered by the key (Phase 12), sized by
            // the connection's row limit.
            maxRows: pageSize.value,
            filters: request,
            // The key must be known first: paging without it would order
            // the first page differently from the rest.
            primaryKey: await editing
              .keyReady()
              .then(() => editing.primaryKey.value),
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
/** The page just read, and the estimate of how many rows the table has. */
const page = computed(() =>
  current.value.status === "loaded" ? current.value.value : null,
);

const previewSource = computed(() =>
  tab.value === "data" && page.value
    ? tableSource({
        profileId: props.tab.profileId ?? explorer.profile.value?.id,
        tabId: props.tab.id,
        target: node.value,
        filters: toRequest(filters.value),
        primaryKey: editing.primaryKey.value,
        estimate: page.value.estimatedTotal ?? null,
        // The live list, not a snapshot: the grid and the editor must agree.
        columns,
        pageSize: pageSize.value,
        firstRows: page.value.rows,
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
      :presets="presetMenu"
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
    <template v-else-if="previewSource">
      <ResultView
        ref="gridView"
        class="min-h-0 flex-1"
        :source="previewSource"
        :result="current.value"
        :editor="editing.editable.value ? editor : null"
        :layout="columnLayout"
        :note="
          editing.readOnlyReason.value ||
          (editing.editable.value ? 'Double-click a cell to edit' : '')
        "
      />
      <TablePager
        :page="pager.page"
        :page-size="pager.pageSize"
        :total="pager.total"
        :estimated="pager.estimated"
        :has-more="pager.hasMore"
        class="h-7 shrink-0 border-t border-line bg-toolbar px-3"
        @go="goToPage"
      />
    </template>
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

<script setup>
import { computed, ref, watch } from "vue";
import { useExplorer } from "../../composables/useExplorer.js";
import { useToasts } from "../../composables/useToasts.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { describeObject, openObjectData } from "../../lib/tauri/commands.js";
import ResultView from "../grid/ResultView.vue";
import TablePager from "../grid/TablePager.vue";
import { columnLayoutRef } from "../../composables/useColumns.js";
import { tableSource } from "../../lib/gridSources.js";
import { pagerProps, tablePageSize } from "../../lib/tablePaging.js";
import AddRowDialog from "./AddRowDialog.vue";
import CreateTableDialog from "./CreateTableDialog.vue";
import ChangesPreviewDialog from "./ChangesPreviewDialog.vue";
import EditChangesBar from "./EditChangesBar.vue";
import FilterBar from "./FilterBar.vue";
import ObjectHeader from "./ObjectHeader.vue";
import ObjectLoadState from "./ObjectLoadState.vue";
import { useTableEditing } from "./useTableEditing.js";
import { useTableFilters } from "./useTableFilters.js";
import { useTableLinks } from "./useTableLinks.js";
import StructureEditor from "./StructureEditor.vue";
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
const tab = computed({
  get: () => props.tab.view ?? "data",
  set: (value) => (props.tab.view = value),
});

const results = ref({}); // tab -> { status, value?, error? }
// Columns of the last rows read, so the bar keeps working after an error.
const columns = ref([]);
const profileId = computed(
  () => props.tab.profileId ?? explorer.profile.value?.id,
);
const tableFilters = useTableFilters({
  tab: props.tab,
  node,
  profileId,
  columns,
  view: tab,
  reload: () => load("data"),
  toasts,
});
const { filters, appliedCount } = tableFilters;

// Column layout lives on the tab, so a table remembers it next time.
const columnLayout = columnLayoutRef(props.tab);

const editing = useTableEditing({
  tab: props.tab,
  node,
  profileId,
  columns,
  reload: () => load("data"),
});
const editor = editing.editor;
const links = useTableLinks({
  node,
  profileId,
  columnInfo: editing.columnInfo,
});
const addingRow = ref(false);
const creatingTable = ref(false);
/**
 * Whether the structure may be changed here. A view has no structure to
 * design, and a read-only connection takes no writes at all — offering an
 * editor that would be refused at the server would be worse than not offering.
 */
const canEditStructure = computed(
  () => profile.value?.safeMode !== "read-only",
);
// Save (Mod+S) in this tab applies the staged edits.
props.tab.applyEdits = editing.apply;

async function load(which = tab.value) {
  const target = node.value;
  const id = props.tab.profileId ?? explorer.profile.value?.id;
  if (!target || !id) return;
  results.value = { ...results.value, [which]: { status: "loading" } };
  const request = which === "data" ? tableFilters.request() : [];
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
// A foreign key jump re-filters an open tab and asks for a reload.
watch(
  () => props.tab.reloadToken,
  () => load("data"),
);
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
        filters: tableFilters.request(),
        primaryKey: editing.primaryKey.value,
        estimate: page.value.estimatedTotal ?? null,
        // The live list, not a snapshot: the grid and the editor must agree.
        columns,
        pageSize: pageSize.value,
        firstRows: page.value.rows,
      })
    : null,
);
const gridView = ref(null);

/** What the pager shows; see lib/tablePaging.js. */
const pager = computed(() =>
  pagerProps(
    gridView.value?.paging ?? null,
    previewSource.value,
    pageSize.value,
  ),
);
const goToPage = (index) => gridView.value?.goToPage(index);
</script>

<template>
  <section v-if="node" class="flex h-full min-h-0 flex-col">
    <ObjectHeader
      v-model:view="tab"
      :node="node"
      :can-add-row="editing.editable.value && columns.length > 0"
      :filters-shown="!!props.tab.showFilters"
      :applied-count="appliedCount"
      :loading="current.status === 'loading'"
      @add-row="addingRow = true"
      @toggle-filters="tableFilters.toggle"
      @reload="load()"
    />
    <FilterBar
      v-if="tableFilters.show.value && columns.length"
      v-model:filters="filters"
      :columns="columns.map((c) => c.name)"
      :applied-count="appliedCount"
      :loading="current.status === 'loading'"
      :presets="tableFilters.presetMenu.value"
      :preview-sql="tableFilters.previewSql"
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
    <ObjectLoadState
      v-if="['loading', 'idle', 'error'].includes(current.status)"
      :state="current"
      :view="tab"
      :profile-id="profileId"
      @retry="load()"
    />
    <template v-else-if="previewSource">
      <ResultView
        ref="gridView"
        v-model:layout="columnLayout"
        class="min-h-0 flex-1"
        :source="previewSource"
        :result="current.value"
        :editor="editing.editable.value ? editor : null"
        :quick-filter="node.kind === 'table' ? tableFilters.quickFilter : null"
        :cell-links="links.cellLinks"
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
    <!-- A view has nothing to design, and a read-only connection can change
         nothing, so those keep the read-only structure view. -->
    <StructureEditor
      v-else-if="node.kind === 'table' && canEditStructure"
      class="min-h-0 flex-1"
      :tab="props.tab"
      :structure="current.value"
      :profile-id="profileId ?? ''"
      :database="node.database ?? ''"
      :schema="node.schema ?? ''"
      :title="node.name"
      @applied="load('structure')"
    />
    <StructureView
      v-else
      class="min-h-0 flex-1"
      :structure="current.value"
      :kind="node.kind"
    />
    <CreateTableDialog
      v-model:open="creatingTable"
      :profile-id="props.tab.profileId ?? ''"
      :tab-id="props.tab.id"
      :database="node.database ?? ''"
      :schema="node.schema ?? ''"
      :engine="
        connections.profiles.value.find((p) => p.id === props.tab.profileId)
          ?.driver ?? 'postgres'
      "
      @created="load('structure')"
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

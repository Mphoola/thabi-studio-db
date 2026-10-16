<script setup>
import { computed, ref } from "vue";
import { ChartColumn, Columns3, Download } from "lucide-vue-next";
import { EXPORT_FORMATS } from "../../lib/tauri/files.js";
import { resultFooter } from "../../lib/resultFooter.js";
import { PAGE_SIZE } from "../../composables/useGridData.js";
import { useColumns } from "../../composables/useColumns.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail } from "../../lib/errors.js";
import { showInFileManager } from "../../lib/tauri/files.js";
import { words } from "../../lib/platform.js";
import BaseButton from "../common/BaseButton.vue";
import DropdownMenu from "../common/DropdownMenu.vue";
import ColumnsDialog from "./ColumnsDialog.vue";
import ResultChart from "./ResultChart.vue";
import ResultGrid from "./ResultGrid.vue";

/** A result grid with its summary bar: rows, time, limits and export. */
const props = defineProps({
  source: { type: Object, required: true },
  result: { type: Object, required: true },
  /** Extra note, e.g. "Read-only preview". */
  note: { type: String, default: "" },
  /** Staged row edits (useRowEdits); makes the grid editable. */
  editor: { type: Object, default: null },
  /** Column order and hidden columns, kept by the caller (v-model:layout). */
  layout: { type: Object, default: null },
  /** (columnName, cell, exclude) => void; offers quick filters when given. */
  quickFilter: { type: Function, default: null },
  /** (columnName, cell) => { label, run }[]: foreign key jumps for a cell. */
  cellLinks: { type: Function, default: null },
});
const emit = defineEmits(["update:layout"]);
const toasts = useToasts();

const footer = computed(() => resultFooter(props.source, props.result));

/** Format for the export; null keeps the source's own default (CSV). */
const columns = useColumns({
  columns: computed(() => props.source.columns),
  // Writable so hiding or moving a column is remembered by the caller.
  layout: computed({
    get: () => props.layout,
    set: (value) => emit("update:layout", value),
  }),
});
const columnsOpen = ref(false);
// Chart instead of the grid; offered for query results, not tables.
const charting = ref(false);
const grid = ref(null);

/**
 * Paging, for a caller that puts the pager somewhere other than here — a table
 * view does, since a query result is capped and pages through what it fetched.
 *
 * One object rather than three properties: these cross between components, and
 * a caller that reads one of them wrongly is not caught by anything. They travel
 * together so they cannot be mixed up or half-read.
 */
const paging = computed(() => ({
  page: grid.value?.page ?? 0,
  pageSize: grid.value?.pageSize() ?? PAGE_SIZE,
  hasMore: grid.value?.hasMore() ?? false,
}));
const goToPage = (index) => grid.value?.goToPage(index);
defineExpose({ paging, goToPage });

const format = ref(null);
const exportMenu = ref(null);
const formatItems = EXPORT_FORMATS.map((entry) => ({
  id: entry.value,
  label: entry.label,
}));

function chooseFormat(value) {
  format.value = value;
  exportCsv();
}

async function exportCsv() {
  try {
    const done = await props.source.export(
      { includeHeader: true },
      null,
      format.value,
    );
    if (!done) return;
    const file = done.path.split("/").pop();
    toasts.success(
      `Exported ${done.rows.toLocaleString()} rows to ${file}`,
      "",
      {
        label: words.fileManager,
        run: () => showInFileManager(done.path),
      },
    );
  } catch (error) {
    toasts.error("Couldn’t export", toUserDetail(error));
  }
}
</script>

<template>
  <div class="flex min-h-0 flex-col">
    <ResultChart v-if="charting" :source="source" class="min-h-0 flex-1" />
    <ResultGrid
      v-show="!charting"
      ref="grid"
      :source="source"
      :editor="editor"
      :column-indexes="columns.visible.value"
      :quick-filter="quickFilter"
      :cell-links="cellLinks"
      class="min-h-0 flex-1"
    />
    <footer
      class="flex h-7 shrink-0 items-center gap-2 border-t border-line bg-toolbar px-3 text-xs whitespace-nowrap text-fg-muted"
    >
      <span v-if="footer.summary">{{ footer.summary }}</span>
      <span v-if="footer.elapsedMs">· {{ footer.elapsedMs }}</span>
      <span
        v-for="warning in footer.warnings"
        :key="warning"
        class="text-warning-text"
        >· {{ warning }}</span
      >
      <span v-if="note" class="min-w-0 truncate" :title="note"
        >· {{ note }}</span
      >
      <BaseButton
        size="sm"
        variant="ghost"
        :icon="Columns3"
        @click="columnsOpen = true"
      >
        Columns
      </BaseButton>
      <BaseButton
        v-if="source.kind === 'result'"
        size="sm"
        variant="ghost"
        :icon="ChartColumn"
        :aria-pressed="charting"
        @click="charting = !charting"
      >
        {{ charting ? "Grid" : "Chart" }}
      </BaseButton>
      <BaseButton
        v-if="source.export"
        class="ml-auto"
        size="sm"
        variant="ghost"
        :icon="Download"
        @click="(event) => exportMenu?.openAt(event.clientX, event.clientY)"
      >
        Export…
      </BaseButton>
      <DropdownMenu
        ref="exportMenu"
        :items="formatItems"
        label="Export format"
        @select="chooseFormat"
      />
    </footer>
    <ColumnsDialog
      v-model:open="columnsOpen"
      :columns="source.columns"
      :visible="columns.visible.value"
      :is-hidden="columns.isHidden"
      @toggle="columns.toggle"
      @move="columns.move"
      @reset="columns.reset"
      @fit="grid?.fitToContents()"
    />
  </div>
</template>

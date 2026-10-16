<script setup>
import { computed, onBeforeUnmount, onMounted, ref, toRef } from "vue";
import { useGridData } from "../../composables/useGridData.js";
import { useGridSelection } from "../../composables/useGridSelection.js";
import { isNumericType } from "../../lib/gridText.js";
import DropdownMenu from "../common/DropdownMenu.vue";
import GridHeader from "./GridHeader.vue";
import GridRow from "./GridRow.vue";
import RowDetailDialog from "./RowDetailDialog.vue";
import ValueInspector from "./ValueInspector.vue";
import { useGridActions } from "./useGridActions.js";
import { useColumnWidths } from "./useColumnWidths.js";
import { useVirtualRows } from "./useVirtualRows.js";
import { gridKeydown } from "./gridKeydown.js";
import { useGridEditing } from "./useGridEditing.js";
import { useRowDetail } from "../../composables/useRowDetail.js";

/**
 * Virtualised results grid, read-only unless an `editor` (useRowEdits) is
 * given. Only the rows on screen are
 * rendered and only nearby pages are fetched from `source`:
 * { columns, total, firstRows, sortable, load(offset, limit, sort),
 *   copy(scope, format, sort), export?(scope, sort) }.
 */
const props = defineProps({
  source: { type: Object, required: true },
  editor: { type: Object, default: null },
  /** Source column indexes to show; all of them when not given. */
  columnIndexes: { type: Array, default: null },
  /** (columnName, cell, exclude) => void; adds "Filter by This Value". */
  quickFilter: { type: Function, default: null },
  /** (columnName, cell) => { label, run }[]: foreign key jumps for a cell. */
  cellLinks: { type: Function, default: null },
});

const ROW = 26;
const OVERSCAN = 8;
const NUMBER_COL = 52;

const sourceRef = toRef(props, "source");
const data = useGridData(sourceRef);

/**
 * Source column indexes the grid renders, left to right. Everything inside
 * the grid speaks display positions; `visible` maps them back to the source
 * column so selection, editing and copy keep addressing the right one.
 */
const columnIndexes = computed(
  () => props.columnIndexes ?? props.source.columns.map((_, index) => index),
);
const columns = computed(() =>
  columnIndexes.value.map((index) => props.source.columns[index]),
);
// The scroller is as tall as what is loaded, not as tall as the table. A
// million rows would be twenty-six million pixels of spacer, past what a
// browser will lay out, and holding the rows behind it would be the same
// mistake a page size is meant to avoid.
const rowCount = computed(() => data.loaded.value);
const selection = useGridSelection(
  rowCount,
  computed(() => columns.value.length),
);
const numeric = computed(() =>
  columns.value.map((c) => isNumericType(c.typeName)),
);
const { widths, fitToContents, resizeColumn, template } = useColumnWidths({
  columns,
  columnIndexes,
  data,
  numberColumn: NUMBER_COL,
});
const scroller = ref(null);
const { scrollTop, viewport, page, goToPage, visible } = useVirtualRows({
  data,
  rowCount,
  scroller,
  rowHeight: ROW,
  overscan: OVERSCAN,
});

const actions = useGridActions(props, data, selection);

const menu = ref(null);
const inspector = ref(null);
const detailOpen = ref(false);

const {
  onEditKey,
  editing,
  startEdit,
  commitEdit,
  endEdit,
  toggleDeleteSelected,
  onMenuSelect,
} = useGridEditing({
  props,
  data,
  selection,
  scroller,
  openInspector,
  actions,
});

const onKeydown = gridKeydown({
  editing,
  selection,
  viewport,
  rowHeight: ROW,
  scroller,
  onEditKey,
  actions,
  startEdit,
  openInspector,
  openDetail,
  toggleDeleteSelected,
  editable: () => Boolean(props.editor?.enabled.value),
});

function openInspector() {
  const { row, col } = selection.focus.value;
  const cells = data.rowAt(row);
  if (!cells) return;
  inspector.value?.open(columns.value[col], shownCell(cells, col));
}

/** The row as a whole form, for reading or editing a record properly. */
function openDetail() {
  const { row } = selection.focus.value;
  if (data.rowAt(row)) detailOpen.value = true;
}
/**
 * A cell as the value inspector and the detail panel want it. `position` is a
 * display position; the editor and the row are addressed by source column.
 */
function shownCell(cells, position) {
  const column = columnIndexes.value[position];
  const cell = props.editor
    ? props.editor.display(cells, column)
    : cells?.[column];
  // Same rule as a grid row: never hand back nothing.
  return cell ?? { kind: "null" };
}

function onCellDown(event, row, col) {
  if (event.button === 2 && selection.isSelected(row, col)) return;
  selection.select(row, col, event.shiftKey);
}

function onContextMenu(event) {
  event.preventDefault();
  menu.value?.openAt(event.clientX, event.clientY);
}

const resize = resizeColumn;

// Lets the command palette open the panel without a ref through the tree.
const rowDetail = useRowDetail();
onMounted(() => rowDetail.set(openDetail));
onBeforeUnmount(() => rowDetail.set(null));

defineExpose({
  fitToContents,
  page,
  goToPage,
  pageSize: data.pageSize,
  hasMore: () => data.hasMore.value,
});
</script>

<template>
  <div class="relative flex min-h-0 flex-col">
    <div
      ref="scroller"
      class="selectable min-h-0 flex-1 overflow-auto font-mono text-sm outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-inset"
      tabindex="0"
      role="grid"
      :aria-rowcount="rowCount + 1"
      :aria-colcount="columns.length"
      aria-label="Results"
      @scroll="scrollTop = $event.target.scrollTop"
      @keydown="onKeydown"
      @contextmenu="onContextMenu"
    >
      <GridHeader
        :columns="columns"
        :visible="columnIndexes"
        :template="template"
        :widths="widths"
        :sort="data.sort.value"
        :sortable="source.sortable !== false"
        @sort="data.toggleSort"
        @resize="resize"
      />
      <div class="relative" :style="{ height: `${rowCount * ROW}px` }">
        <GridRow
          v-for="row in visible"
          :key="row.index"
          :index="row.index"
          :cells="row.cells"
          :visible="columnIndexes"
          :template="template"
          :numeric="numeric"
          :selection="selection"
          :editor="editor"
          :editing-col="editing?.row === row.index ? editing.col : -1"
          @celldown="onCellDown"
          @celldblclick="(r, c) => startEdit(r, c) || openInspector()"
          @commit="commitEdit"
          @cancel="endEdit('stay')"
        />
      </div>
    </div>
    <p
      v-if="data.error.value"
      class="border-t border-line bg-danger-subtle px-3 py-1 text-sm text-danger-text"
      role="alert"
    >
      {{ data.error.value.message ?? "Couldn’t load more rows." }}
    </p>
    <DropdownMenu
      ref="menu"
      :items="actions.menuItems.value"
      label="Result actions"
      @select="onMenuSelect"
    />
    <ValueInspector ref="inspector" />
    <RowDetailDialog
      v-model:open="detailOpen"
      :columns="columns"
      :column-indexes="columnIndexes"
      :cells="data.rowAt(selection.focus.value.row)"
      :row="selection.focus.value.row + 1"
      :editor="editor"
    />
  </div>
</template>

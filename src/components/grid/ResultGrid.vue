<script setup>
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  toRef,
  watch,
} from "vue";
import { useGridData } from "../../composables/useGridData.js";
import { useGridSelection } from "../../composables/useGridSelection.js";
import { displayText, isNumericType } from "../../lib/gridText.js";
import DropdownMenu from "../common/DropdownMenu.vue";
import GridHeader from "./GridHeader.vue";
import ValueInspector from "./ValueInspector.vue";
import { useGridActions } from "./useGridActions.js";

/**
 * Virtualised, read-only results grid. Only the rows on screen are
 * rendered and only nearby pages are fetched from `source`:
 * { columns, total, firstRows, sortable, load(offset, limit, sort),
 *   copy(scope, format, sort), export?(scope, sort) }.
 */
const props = defineProps({ source: { type: Object, required: true } });

const ROW = 26;
const OVERSCAN = 8;
const NUMBER_COL = 52;

const sourceRef = toRef(props, "source");
const data = useGridData(sourceRef);
const columns = computed(() => props.source.columns);
const rowCount = computed(() => props.source.total);
const selection = useGridSelection(
  rowCount,
  computed(() => columns.value.length),
);
const numeric = computed(() =>
  columns.value.map((c) => isNumericType(c.typeName)),
);
const widths = ref([]);
watch(
  columns,
  (cols) =>
    (widths.value = cols.map((c) => (isNumericType(c.typeName) ? 120 : 180))),
  { immediate: true },
);

const scroller = ref(null);
const scrollTop = ref(0);
const viewport = ref(400);
let observer;
onMounted(() => {
  observer = new ResizeObserver(
    ([entry]) => (viewport.value = entry.contentRect.height),
  );
  observer.observe(scroller.value);
});
onBeforeUnmount(() => observer?.disconnect());

const first = computed(() =>
  Math.max(0, Math.floor(scrollTop.value / ROW) - OVERSCAN),
);
const last = computed(() =>
  Math.min(
    rowCount.value,
    Math.ceil((scrollTop.value + viewport.value) / ROW) + OVERSCAN,
  ),
);
const visible = computed(() => {
  void data.version.value;
  const rows = [];
  for (let i = first.value; i < last.value; i += 1)
    rows.push({ index: i, cells: data.rowAt(i) });
  return rows;
});
const template = computed(
  () => `${NUMBER_COL}px ${widths.value.map((w) => `${w}px`).join(" ")}`,
);

const actions = useGridActions(props, data, selection);
const menu = ref(null);
const inspector = ref(null);

function scrollIntoView(row) {
  const el = scroller.value;
  const top = row * ROW;
  const header = ROW;
  if (top < el.scrollTop) el.scrollTop = top;
  else if (top + ROW + header > el.scrollTop + el.clientHeight)
    el.scrollTop = top + ROW + header - el.clientHeight;
}

function onKeydown(event) {
  if (
    selection.move(event, Math.max(1, Math.floor(viewport.value / ROW) - 1))
  ) {
    event.preventDefault();
    nextTick(() => scrollIntoView(selection.focus.value.row));
    return;
  }
  if (event.metaKey && event.key.toLowerCase() === "a") {
    event.preventDefault();
    selection.selectAll();
  } else if (event.metaKey && event.key.toLowerCase() === "c") {
    event.preventDefault();
    actions.copy("tsv", event.shiftKey);
  } else if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    openInspector();
  }
}

function openInspector() {
  const { row, col } = selection.focus.value;
  const cells = data.rowAt(row);
  if (cells) inspector.value?.open(columns.value[col], cells[col]);
}

function onCellDown(event, row, col) {
  if (event.button === 2 && selection.isSelected(row, col)) return;
  selection.select(row, col, event.shiftKey);
}

function onContextMenu(event) {
  event.preventDefault();
  menu.value?.openAt(event.clientX, event.clientY);
}

function resize(index, width) {
  widths.value[index] = Math.min(800, Math.max(60, width));
}
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
        :template="template"
        :widths="widths"
        :sort="data.sort.value"
        :sortable="source.sortable !== false"
        @sort="data.toggleSort"
        @resize="resize"
      />
      <div class="relative" :style="{ height: `${rowCount * ROW}px` }">
        <div
          v-for="row in visible"
          :key="row.index"
          role="row"
          :aria-rowindex="row.index + 2"
          class="absolute left-0 grid min-w-full"
          :class="row.index % 2 && 'bg-grid-row-alt'"
          :style="{
            top: `${row.index * ROW}px`,
            height: `${ROW}px`,
            gridTemplateColumns: template,
          }"
        >
          <span
            class="sticky left-0 z-[1] border-r border-line bg-grid-header px-2 text-right leading-[26px] text-fg-disabled tabular-nums"
          >
            {{ (row.index + 1).toLocaleString() }}
          </span>
          <template v-if="row.cells">
            <span
              v-for="(cell, col) in row.cells"
              :key="col"
              role="gridcell"
              class="truncate border-r border-line px-2.5 leading-[26px]"
              :class="[
                numeric[col] &&
                  cell.kind === 'value' &&
                  'text-right tabular-nums',
                cell.kind === 'null' && 'text-grid-null italic',
                cell.kind === 'binary' && 'text-fg-muted',
                cell.kind === 'value' && cell.value === '' && 'text-fg-subtle',
                selection.isSelected(row.index, col) && 'bg-selection',
                selection.focus.value.row === row.index &&
                  selection.focus.value.col === col &&
                  selection.active.value &&
                  'outline-2 -outline-offset-2 outline-accent',
              ]"
              @mousedown="onCellDown($event, row.index, col)"
              @dblclick="openInspector"
            >
              {{
                cell.kind === "value" && cell.value === ""
                  ? "''"
                  : displayText(cell)
              }}
            </span>
          </template>
          <span v-else class="px-2.5 leading-[26px] text-fg-subtle"
            >Loading…</span
          >
        </div>
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
      @select="(id) => (id === 'inspect' ? openInspector() : actions.run(id))"
    />
    <ValueInspector ref="inspector" />
  </div>
</template>

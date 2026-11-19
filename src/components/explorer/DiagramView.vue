<script setup>
/**
 * The ER diagram: one box per table, one line per foreign key.
 *
 * Read-only, and the header says so, so nobody reads a diagram as an
 * invitation to edit.
 *
 * Holds the data and the gestures. DiagramCanvas draws, DiagramSearch lists,
 * DiagramToolbar is the header, DiagramStates covers what is shown instead of a
 * drawing, and useDiagramZoom holds the pan and zoom.
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useDiagramZoom } from "../../composables/useDiagramZoom.js";
import { useDiagramData } from "../../composables/useDiagramData.js";
import { useDiagramDrag } from "../../composables/useDiagramDrag.js";
import { useDiagramExport } from "../../composables/useDiagramExport.js";
import { rowsFor } from "../../lib/erSvg.js";
import { ROW_LIMIT } from "../../lib/erLayout.js";
import DiagramCanvas from "./DiagramCanvas.vue";
import DiagramSearch from "./DiagramSearch.vue";
import DiagramStates from "./DiagramStates.vue";
import DiagramToolbar from "./DiagramToolbar.vue";

defineOptions({ name: "DiagramView" });

const props = defineProps({
  tab: { type: Object, required: true },
  // The profile, not the connection: every object command is keyed by
  // profile id, and a connection id finds no session and reads as
  // "not connected" with no other symptom.
  profileId: { type: String, default: null },
});
const emit = defineEmits(["open-table"]);

/** Columns shown per table, so a 400-column table is still a box. */
const MAX_ROWS = ROW_LIMIT;

const hovered = ref(null);
const selected = ref(null);
/** Tables whose every column is drawn, rather than the first few. */
const expanded = ref(new Set());
/** The list is collapsible: the drawing is the point, not the list. */
const listing = ref(true);

const view = useDiagramZoom();
const late = {};
const { host, zoom, offset, panning, extent } = view;
const {
  loading,
  failed,
  tables,
  foreignKeys,
  keyed,
  nodes,
  lines,
  laidOut,
  caption,
  emptyReason,
  load,
  rearrange,
} = useDiagramData({
  tab: props.tab,
  profileId: () => props.profileId,
  view,
  limitFor: (name) => (expanded.value.has(name) ? Infinity : MAX_ROWS),
  // The drag is set up from this data, so it's reached late.
  afterLayout: () => late.drag?.reapply(),
});
const drag = (late.drag = useDiagramDrag({
  tab: props.tab,
  nodes,
  lines,
  zoom,
  view,
}));

/** Columns by table, so the list can search column names as well as tables. */
const columnsByTable = computed(() =>
  Object.fromEntries(
    tables.value.map((table) => [
      table.name,
      table.columns.map((column) => column.name),
    ]),
  ),
);

/**
 * Each box is capped so a wide table is still a box, unless someone asks to
 * see the rest — a cap with no way past it hides columns for good.
 */
const trimmed = computed(() =>
  tables.value.map((table) => ({
    name: table.name,
    ...rowsFor(
      tables.value,
      table.name,
      expanded.value.has(table.name) ? Infinity : MAX_ROWS,
    ),
  })),
);

/** Picks a table from the list: selects it and brings it into view. */
function focusTable(name) {
  const node = nodes.value[name];
  if (!node) return;
  selected.value = name;
  view.centreOn({ x: node.x + node.width / 2, y: node.y + node.height / 2 });
}

/** Clicking a table in the drawing selects it, or clears it, in place. */
function toggleSelect(name) {
  if (drag.tookClick()) return;
  selected.value = selected.value === name ? null : name;
}

/**
 * Shows every column of a table, and rearranges, because the box changes
 * height and the lines attached to it would otherwise point at the wrong rows.
 */
function toggleExpand(name) {
  const next = new Set(expanded.value);
  if (next.has(name)) next.delete(name);
  else next.add(name);
  expanded.value = next;
  rearrange();
}

const exportAs = useDiagramExport({
  title: () => props.tab.title,
  extent,
  nodes,
  lines,
  tables: trimmed,
  keyed,
});

defineExpose({ reload: load });

watch(() => [props.tab.database, props.tab.schema, props.profileId], load);
onMounted(load);
onBeforeUnmount(view.panEnd);
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <DiagramToolbar
      :title="tab.title"
      :tables="trimmed.length"
      :keys="foreignKeys.length"
      :zoom-percent="Math.round(zoom * 100)"
      :loading="loading"
      :can-export="Boolean(extent.width)"
      :moved="drag.moved.value"
      @zoom-out="view.zoomBy(1 / 1.1)"
      @zoom-in="view.zoomBy(1.1)"
      @fit="view.fit"
      @export="exportAs"
      @reload="load"
      @reset-layout="drag.reset() || rearrange()"
    />

    <div class="flex min-h-0 flex-1">
      <aside
        v-if="listing"
        class="w-52 shrink-0 border-r border-line bg-surface"
      >
        <DiagramSearch
          :tables="trimmed"
          :columns-by-table="columnsByTable"
          :selected="selected"
          @select="focusTable"
        />
      </aside>

      <div ref="host" class="bg-bg relative min-h-0 flex-1 overflow-hidden">
        <DiagramStates
          :failed="failed"
          :loading="loading"
          :empty-reason="emptyReason"
          :arranging="!laidOut"
          :has-tables="Boolean(tables.length)"
          :database="tab.database"
          :schema="tab.schema"
        />
        <DiagramCanvas
          v-if="tables.length"
          :nodes="nodes"
          :lines="lines"
          :tables="trimmed"
          :keyed="keyed"
          :caption="caption"
          :expanded="expanded"
          :hovered="hovered"
          :selected="selected"
          :title="tab.title"
          :zoom="zoom"
          :offset="offset"
          :panning="panning"
          @open-table="emit('open-table', $event)"
          @hover="hovered = $event"
          @select="toggleSelect"
          @expand="toggleExpand"
          @drag-start="drag.start"
          @pan-start="view.panStart"
          @pan-move="drag.move($event) || view.panMove($event)"
          @pan-end="drag.end() || view.panEnd()"
          @zoom-wheel="view.wheel"
        />
        <button
          type="button"
          class="absolute top-2 left-2 rounded border border-line bg-surface px-1.5 py-0.5 text-xs text-fg-muted hover:text-fg"
          @click="listing = !listing"
        >
          {{ listing ? "Hide tables" : "Show tables" }}
        </button>
        <p
          v-if="tables.length"
          class="pointer-events-none absolute right-3 bottom-2 text-xs text-fg-subtle"
        >
          Drag or scroll to move · pinch or ⌘ + scroll to zoom · double-click a
          table to open it
        </p>
      </div>
    </div>
  </div>
</template>

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
import { useToasts } from "../../composables/useToasts.js";
import { useDiagramZoom } from "../../composables/useDiagramZoom.js";
import { whyEmpty } from "../../composables/useDiagramEmpty.js";
import { schemaDiagram } from "../../lib/tauri/commands.js";
import { toUserDetail } from "../../lib/errors.js";
import { buildGraph, plainGraph, positionsFrom } from "../../lib/erLayout.js";
import {
  buildSvg,
  downloadSvg,
  fileNameFor,
  rowsFor,
} from "../../lib/erSvg.js";
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
const MAX_ROWS = 14;

const toasts = useToasts();
const loading = ref(false);
const failed = ref("");
const tables = ref([]);
const foreignKeys = ref([]);
const nodes = ref({});
const lines = ref([]);
const laidOut = ref(false);
const hovered = ref(null);
const selected = ref(null);
/** Tables whose every column is drawn, rather than the first few. */
const expanded = ref(new Set());
/** The list is collapsible: the drawing is the point, not the list. */
const listing = ref(true);
/** Why nothing came back, worked out rather than guessed at. */
const emptyReason = ref("");

const view = useDiagramZoom();
const { host, zoom, offset, panning, extent } = view;
let elk = null;

/** Tables as drawn: columns capped, the rest counted rather than dropped. */
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

async function load() {
  if (!props.profileId) {
    failed.value = "This connection is not open.";
    return;
  }
  loading.value = true;
  failed.value = "";
  laidOut.value = false;
  try {
    const diagram = await schemaDiagram(
      props.profileId,
      props.tab.database,
      props.tab.schema,
    );
    tables.value = diagram.tables ?? [];
    foreignKeys.value = diagram.foreignKeys ?? [];
    emptyReason.value = "";
    if (!tables.value.length) emptyReason.value = await whyEmpty();
    const graph = buildGraph(diagram);
    // Something correct on screen while elk works out where things go.
    const plain = plainGraph(graph);
    nodes.value = plain.nodes;
    lines.value = plain.lines;
    view.setNodes(plain.nodes);
    await arrange(graph);
  } catch (failure) {
    failed.value = toUserDetail(failure);
  } finally {
    loading.value = false;
  }
}

async function arrange(graph) {
  try {
    if (!elk) {
      const ELK = (await import("elkjs/lib/elk.bundled.js")).default;
      elk = new ELK();
    }
    const placed = positionsFrom(await elk.layout(graph));
    nodes.value = placed.nodes;
    lines.value = placed.lines;
    view.setNodes(placed.nodes);
    laidOut.value = true;
  } catch (failure) {
    // The boxes are already drawn and correct; only the arrangement failed, so
    // say so rather than replacing a usable picture with an error.
    laidOut.value = false;
    toasts.error("Couldn't arrange the diagram", toUserDetail(failure));
  }
  view.fit();
}

function exportSvg() {
  if (!extent.value.width) return;
  downloadSvg(
    buildSvg(
      { ...extent.value, nodes: nodes.value, lines: lines.value },
      trimmed.value,
    ),
    fileNameFor(props.tab.title),
  );
}

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
      @zoom-out="view.zoomBy(1 / 1.1)"
      @zoom-in="view.zoomBy(1.1)"
      @fit="view.fit"
      @export="exportSvg"
      @reload="load"
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
          :expanded="expanded"
          :hovered="hovered"
          :selected="selected"
          :title="tab.title"
          :zoom="zoom"
          :offset="offset"
          :panning="panning"
          @open-table="emit('open-table', $event)"
          @hover="hovered = $event"
          @select="focusTable"
          @expand="toggleExpand"
          @pan-start="view.panStart"
          @pan-move="view.panMove"
          @pan-end="view.panEnd"
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
          drag to pan · ⌘/Ctrl + scroll to zoom · click a table to focus
        </p>
      </div>
    </div>
  </div>
</template>

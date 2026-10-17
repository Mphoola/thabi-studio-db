<script setup>
/**
 * The ER diagram: one box per table, one line per foreign key.
 *
 * Read-only. Nothing here can change the database, and the header says so, so
 * nobody reads a diagram as an invitation to edit.
 *
 * This holds the data and the gestures. The drawing is DiagramCanvas and the
 * exported file is lib/erSvg.js, both of which are given the nodes rather
 * than the diagram, so neither can disagree with the other about geometry.
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  Download,
  Minus,
  Network,
  Plus,
  RefreshCw,
  Table2,
} from "lucide-vue-next";
import BaseButton from "../common/BaseButton.vue";
import DiagramCanvas from "./DiagramCanvas.vue";
import EmptyState from "../common/EmptyState.vue";
import IconButton from "../common/IconButton.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";
import { useToasts } from "../../composables/useToasts.js";
import { listDatabases, schemaDiagram } from "../../lib/tauri/commands.js";
import { toUserDetail } from "../../lib/errors.js";
import { buildGraph, plainGraph, positionsFrom } from "../../lib/erLayout.js";
import { useDiagramZoom } from "../../composables/useDiagramZoom.js";
import { buildSvg, fileNameFor, rowsFor } from "../../lib/erSvg.js";

defineOptions({ name: "DiagramView" });

const props = defineProps({
  tab: { type: Object, required: true },
  connectionId: { type: String, default: null },
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
/** Why nothing came back, worked out rather than guessed at. */
const emptyReason = ref("");

const view = useDiagramZoom();
const { host, zoom, offset, panning, extent } = view;
let elk = null;

/** Tables as drawn: columns capped, the rest counted rather than dropped. */
const trimmed = computed(() =>
  tables.value.map((table) => ({
    name: table.name,
    ...rowsFor(tables.value, table.name, MAX_ROWS),
  })),
);

async function load() {
  if (!props.connectionId) {
    failed.value = "This connection is not open.";
    return;
  }
  loading.value = true;
  failed.value = "";
  laidOut.value = false;
  try {
    const diagram = await schemaDiagram(
      props.connectionId,
      props.tab.database,
      props.tab.schema,
    );
    tables.value = diagram.tables ?? [];
    foreignKeys.value = diagram.foreignKeys ?? [];
    emptyReason.value = "";
    if (!tables.value.length) await explainEmpty();
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

/**
 * Zero tables is not an explanation: the usual cause is the wrong database
 * rather than an empty one. So the panel asks the server whether the database
 * it was pointed at exists, and says which of the two it is.
 */
async function explainEmpty() {
  const asked = props.tab.database;
  try {
    const names = (await listDatabases(props.connectionId)) ?? [];
    emptyReason.value = names.includes(asked)
      ? `“${asked}” is on this connection, but has no tables visible to it.`
      : `There is no database called “${asked}” on this connection. This tab was opened against “${names[0] ?? "nothing"}”.`;
  } catch (failure) {
    emptyReason.value = toUserDetail(failure);
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
  const svg = buildSvg(
    { ...extent.value, nodes: nodes.value, lines: lines.value },
    trimmed.value,
  );
  const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = fileNameFor(props.tab.title);
  link.click();
  URL.revokeObjectURL(url);
}

defineExpose({ reload: load });

watch(() => [props.tab.database, props.tab.schema, props.connectionId], load);
onMounted(load);
onBeforeUnmount(view.panEnd);
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <header
      class="flex shrink-0 items-center gap-1 border-b border-line px-3 py-1.5"
    >
      <Network
        :size="16"
        :stroke-width="1.75"
        class="text-fg-muted"
        aria-hidden="true"
      />
      <span class="mr-1 truncate text-sm font-medium">{{ tab.title }}</span>
      <span class="text-xs text-fg-subtle">
        {{ trimmed.length }} tables · {{ foreignKeys.length }} keys
      </span>
      <span class="ml-2 text-xs text-fg-subtle">Read-only</span>
      <div class="ml-auto flex items-center gap-0.5">
        <IconButton
          :icon="Minus"
          label="Zoom out"
          @click="view.zoomBy(1 / 1.1)"
        />
        <BaseButton size="sm" variant="ghost" @click="view.fit">Fit</BaseButton>
        <IconButton :icon="Plus" label="Zoom in" @click="view.zoomBy(1.1)" />
        <IconButton :icon="Download" label="Export SVG" @click="exportSvg" />
        <IconButton
          :icon="RefreshCw"
          label="Reload"
          :disabled="loading"
          @click="load"
        />
      </div>
    </header>

    <div ref="host" class="bg-bg relative min-h-0 flex-1 overflow-hidden">
      <EmptyState
        v-if="failed"
        :icon="Network"
        title="Couldn’t draw the diagram"
      >
        <!-- The server's own reason, not just "it failed". Repeating the
             message without the detail has cost more time than it saved. -->
        <template #message>{{ failed }}</template>
        <p class="mt-2 text-xs text-fg-subtle">
          {{ tab.database }}{{ tab.schema ? `.${tab.schema}` : "" }} ·
          {{ tab.title }}
        </p>
      </EmptyState>
      <EmptyState
        v-else-if="loading && !tables.length"
        :icon="Network"
        title="Reading the schema…"
      />
      <EmptyState
        v-else-if="!loading && !tables.length"
        :icon="Table2"
        title="Nothing to draw"
      >
        <template #message>
          {{ emptyReason || "No tables came back for this database." }}
        </template>
      </EmptyState>
      <DiagramCanvas
        v-else
        :nodes="nodes"
        :lines="lines"
        :tables="trimmed"
        :title="tab.title"
        :zoom="zoom"
        :offset="offset"
        :panning="panning"
        @open-table="emit('open-table', $event)"
        @pan-start="view.panStart"
        @pan-move="view.panMove"
        @pan-end="view.panEnd"
        @zoom-wheel="view.wheel"
      />
      <div
        v-if="!laidOut && tables.length"
        class="pointer-events-none absolute bottom-2 left-3 flex items-center gap-2 text-xs text-fg-subtle"
      >
        <SpinnerIcon />
        Arranging…
      </div>
      <p
        v-if="tables.length"
        class="pointer-events-none absolute right-3 bottom-2 text-xs text-fg-subtle"
      >
        {{ Math.round(zoom * 100) }}% · drag to pan · ⌘/Ctrl + scroll to zoom
      </p>
    </div>
  </div>
</template>

<script setup>
/**
 * The ER diagram: one box per table, one line per foreign key.
 *
 * Read-only. Nothing here can change the database, and the panel says so, so
 * nobody reads a diagram as an invitation to edit.
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

defineOptions({ name: "DiagramView" });
import {
  Download,
  Minus,
  Network,
  Plus,
  RefreshCw,
  Table2,
} from "lucide-vue-next";
import BaseButton from "../common/BaseButton.vue";
import EmptyState from "../common/EmptyState.vue";
import IconButton from "../common/IconButton.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";
import { useToasts } from "../../composables/useToasts.js";
import { schemaDiagram } from "../../lib/tauri/commands.js";
import { toUserDetail } from "../../lib/errors.js";
import {
  HEADER_HEIGHT,
  ROW_HEIGHT,
  buildGraph,
  extentOf,
  plainGraph,
  positionsFrom,
} from "../../lib/erLayout.js";

const props = defineProps({
  tab: { type: Object, required: true },
  connectionId: { type: String, default: null },
});
const emit = defineEmits(["open-table"]);

const toasts = useToasts();
const loading = ref(false);
const failed = ref("");
const raw = ref(null);
const nodes = ref({});
const lines = ref([]);
const laidOut = ref(false);

const zoom = ref(1);
const offset = ref({ x: 24, y: 24 });
const panFrom = ref(null);
const host = ref(null);
let elk = null;

/** Columns shown per table, so a 400-column table is still a box. */
const MAX_ROWS = 14;

const trimmed = computed(() => {
  const diagram = raw.value;
  if (!diagram) return null;
  return {
    tables: diagram.tables.map((table) => ({
      ...table,
      hidden: Math.max(0, table.columns.length - MAX_ROWS),
      columns: table.columns.slice(0, MAX_ROWS),
    })),
  };
});

const extent = computed(() => extentOf(nodes.value));

/**
 * Fits the drawing to the window.
 *
 * A hundred tables laid out by elk is far wider than any screen, so the first
 * thing to do is make the whole thing visible; zooming in from there is
 * deliberate, where arriving already zoomed to 3% is not.
 */
function fit() {
  const box = host.value?.getBoundingClientRect();
  if (!box || !extent.value.width) return;
  const scale = Math.min(
    box.width / (extent.value.width + 48),
    box.height / (extent.value.height + 48),
    1,
  );
  zoom.value = Number.isFinite(scale) && scale > 0 ? scale : 1;
  offset.value = { x: 24, y: 24 };
}

function zoomBy(factor) {
  zoom.value = Math.min(2, Math.max(0.1, zoom.value * factor));
}

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
    raw.value = diagram;
    const graph = buildGraph(diagram);
    // Something correct on screen while elk works out where things go.
    const plain = plainGraph(graph);
    nodes.value = plain.nodes;
    lines.value = plain.lines;
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
    laidOut.value = true;
    fit();
  } catch (failure) {
    // The boxes are already drawn and correct; only the arrangement failed, so
    // say so rather than replacing a usable picture with an error.
    laidOut.value = false;
    toasts.error("Couldn't arrange the diagram", toUserDetail(failure));
    fit();
  }
}

function onPointerDown(event) {
  panFrom.value = {
    x: event.clientX - offset.value.x,
    y: event.clientY - offset.value.y,
  };
  host.value?.setPointerCapture?.(event.pointerId);
}

function onPointerMove(event) {
  if (!panFrom.value) return;
  offset.value = {
    x: event.clientX - panFrom.value.x,
    y: event.clientY - panFrom.value.y,
  };
}

function onPointerUp() {
  panFrom.value = null;
}

function onWheel(event) {
  // Trackpads send wheel events to scroll, and a diagram that jumps when the
  // pointer crosses it is unusable. Zoom only with the modifier held.
  if (!event.ctrlKey && !event.metaKey) return;
  event.preventDefault();
  zoomBy(event.deltaY < 0 ? 1.1 : 1 / 1.1);
}

/**
 * Writes the drawing to a file.
 *
 * The SVG is built by hand from the same nodes the panel is drawing, so what
 * is exported is what is on screen rather than a second rendering that can
 * drift from the first.
 */
function exportSvg() {
  const { width, height } = extent.value;
  if (!width) return;
  const parts = [];
  for (const line of lines.value) {
    parts.push(
      `<line x1="${line.from.x}" y1="${line.from.y}" x2="${line.to.x}" y2="${line.to.y}" ` +
        `stroke="var(--line)" stroke-width="1.5" marker-end="url(#arrow)"/>`,
    );
  }
  for (const node of Object.values(nodes.value)) {
    const table = trimmed.value?.tables.find((one) => one.name === node.id);
    const rows = (table?.columns ?? [])
      .map((column, index) => {
        const y = node.y + HEADER_HEIGHT + index * ROW_HEIGHT + 13;
        const weight = column.primaryKey ? "700" : "400";
        const fill = column.primaryKey ? "var(--fg)" : "var(--fg-muted)";
        return (
          `<text x="${node.x + 10}" y="${y}" font-size="11" font-weight="${weight}" ` +
          `fill="${fill}">${escapeXml(column.name)}</text>` +
          `<text x="${node.x + node.width - 10}" y="${y}" font-size="10" text-anchor="end" ` +
          `fill="var(--fg-subtle)">${escapeXml(column.dataType)}</text>`
        );
      })
      .join("");
    parts.push(
      `<rect x="${node.x}" y="${node.y}" width="${node.width}" height="${node.height}" ` +
        `rx="6" fill="var(--surface)" stroke="var(--line)"/>`,
      `<rect x="${node.x}" y="${node.y}" width="${node.width}" height="${HEADER_HEIGHT}" ` +
        `rx="6" fill="var(--surface-raised)"/>`,
      `<text x="${node.x + 10}" y="${node.y + 19}" font-size="12" font-weight="600" ` +
        `fill="var(--fg)">${escapeXml(node.id)}</text>`,
      rows,
    );
  }
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" ` +
    `viewBox="0 0 ${width} ${height}">` +
    '<defs><marker id="arrow" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" ' +
    'markerHeight="7" orient="auto"><path d="M0,0 L8,4 L0,8 z" fill="var(--line)"/></marker></defs>' +
    `<rect width="${width}" height="${height}" fill="var(--bg)"/>` +
    parts.join("") +
    "</svg>";
  const blob = new Blob([svg], { type: "image/svg+xml" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = `${props.tab.title.replace(/[^\w.-]+/g, "-")}.svg`;
  link.click();
  URL.revokeObjectURL(url);
}

function escapeXml(text) {
  return String(text ?? "").replace(
    /[<>&"']/g,
    (one) =>
      ({
        "<": "&lt;",
        ">": "&gt;",
        "&": "&amp;",
        '"': "&quot;",
        "'": "&apos;",
      })[one],
  );
}

function openTable(name) {
  emit("open-table", name);
}

defineExpose({ reload: load });

watch(() => [props.tab.database, props.tab.schema, props.connectionId], load);
onMounted(load);
onBeforeUnmount(() => {
  panFrom.value = null;
});

const transform = computed(
  () => `translate(${offset.value.x}, ${offset.value.y}) scale(${zoom.value})`,
);
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
        {{ trimmed?.tables.length ?? 0 }} tables ·
        {{ raw?.foreignKeys?.length ?? 0 }} keys
      </span>
      <span class="ml-2 text-xs text-fg-subtle">Read-only</span>
      <div class="ml-auto flex items-center gap-0.5">
        <IconButton :icon="Minus" label="Zoom out" @click="zoomBy(1 / 1.1)" />
        <BaseButton size="sm" variant="ghost" @click="fit">Fit</BaseButton>
        <IconButton :icon="Plus" label="Zoom in" @click="zoomBy(1.1)" />
        <IconButton :icon="Download" label="Export SVG" @click="exportSvg" />
        <IconButton
          :icon="RefreshCw"
          label="Reload"
          :disabled="loading"
          @click="load"
        />
      </div>
    </header>

    <div
      ref="host"
      class="bg-bg relative min-h-0 flex-1 overflow-hidden select-none"
      :class="panFrom ? 'cursor-grabbing' : 'cursor-grab'"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
      @wheel="onWheel"
    >
      <EmptyState
        v-if="failed"
        :icon="Network"
        title="Couldn't draw the diagram"
      >
        <template #message>{{ failed }}</template>
      </EmptyState>
      <EmptyState
        v-else-if="loading && !trimmed"
        :icon="Network"
        title="Reading the schema…"
      />
      <EmptyState
        v-else-if="!loading && trimmed && !trimmed.tables.length"
        :icon="Table2"
        title="No tables here"
      >
        <template #message>This schema has no tables to draw.</template>
      </EmptyState>

      <svg
        v-else-if="trimmed"
        class="absolute inset-0 h-full w-full"
        :aria-label="`Diagram of ${tab.title}`"
        role="img"
      >
        <g :transform="transform">
          <line
            v-for="line in lines"
            :key="line.id"
            :x1="line.from.x"
            :y1="line.from.y"
            :x2="line.to.x"
            :y2="line.to.y"
            stroke="border-line-strong"
            stroke-width="1.5"
          />
          <g
            v-for="node in Object.values(nodes)"
            :key="node.id"
            :transform="`translate(${node.x}, ${node.y})`"
          >
            <rect
              :width="node.width"
              :height="node.height"
              rx="6"
              class="fill-surface stroke-line"
              stroke-width="1"
            />
            <rect
              :width="node.width"
              :height="HEADER_HEIGHT"
              rx="6"
              class="fill-surface-raised"
            />
            <line
              x1="0"
              :y1="HEADER_HEIGHT"
              :x2="node.width"
              :y2="HEADER_HEIGHT"
              class="stroke-line"
            />
            <text
              x="10"
              :y="HEADER_HEIGHT - 11"
              class="fill-fg text-xs font-semibold"
            >
              {{ node.id }}
            </text>
            <g
              v-for="(column, index) in trimmed.tables.find(
                (one) => one.name === node.id,
              )?.columns ?? []"
              :key="column.name"
              class="cursor-pointer"
              @click.stop="openTable(node.id)"
            >
              <text
                x="10"
                :y="HEADER_HEIGHT + index * ROW_HEIGHT + 13"
                class="text-[11px]"
                :class="
                  column.primaryKey ? 'fill-fg font-bold' : 'fill-fg-muted'
                "
              >
                {{ column.name }}
              </text>
              <text
                :x="node.width - 10"
                :y="HEADER_HEIGHT + index * ROW_HEIGHT + 13"
                text-anchor="end"
                class="fill-fg-subtle text-[10px]"
              >
                {{ column.dataType }}
              </text>
            </g>
            <text
              v-if="trimmed.tables.find((one) => one.name === node.id)?.hidden"
              x="10"
              :y="node.height - 4"
              class="fill-fg-subtle text-[10px]"
            >
              +{{ trimmed.tables.find((one) => one.name === node.id).hidden }}
              more
            </text>
          </g>
        </g>
      </svg>

      <div
        v-if="!laidOut && trimmed"
        class="pointer-events-none absolute bottom-2 left-3 flex items-center gap-2 text-xs text-fg-subtle"
      >
        <SpinnerIcon />
        Arranging…
      </div>
      <p
        v-if="trimmed"
        class="pointer-events-none absolute right-3 bottom-2 text-xs text-fg-subtle"
      >
        {{ Math.round(zoom * 100) }}% · drag to pan · ⌘/Ctrl + scroll to zoom
      </p>
    </div>
  </div>
</template>

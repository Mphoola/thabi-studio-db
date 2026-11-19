<script setup>
/**
 * The drawing surface: the SVG, and the pan and zoom over it.
 *
 * Given nodes and lines, draws them. Knows nothing about databases, so the
 * part that can be wrong about an engine is nowhere near the part that can be
 * wrong about a gesture.
 */
import { computed } from "vue";
import { roundedPath } from "../../lib/erLayout.js";
import {
  lineIsDimmed,
  lineIsLit,
  tableIsDimmed,
} from "../../lib/erHighlight.js";
import { rowsFor } from "../../lib/erSvg.js";
import DiagramTableBox from "./DiagramTableBox.vue";

const props = defineProps({
  /** Positioned boxes, from erLayout. */
  nodes: { type: Object, required: true },
  lines: { type: Array, required: true },
  /** Trimmed tables, so a box knows its own rows. */
  tables: { type: Array, default: () => [] },
  /** Tables whose full column list should be drawn, by name. */
  expanded: { type: Set, default: () => new Set() },
  /** Where the grid of unrelated tables is labelled, or null. */
  caption: { type: Object, default: null },
  /** "table:column" of every foreign key column, for the FK badges. */
  keyed: { type: Set, default: () => new Set() },
  /** Table being pointed at, and the one clicked. */
  hovered: { type: String, default: null },
  selected: { type: String, default: null },
  title: { type: String, required: true },
  zoom: { type: Number, required: true },
  offset: { type: Object, required: true },
  panning: { type: Boolean, default: false },
});
const emit = defineEmits([
  "open-table",
  "drag-start",
  "pan-start",
  "pan-move",
  "pan-end",
  "zoom-wheel",
  "hover",
  "select",
  "expand",
]);

/** Who the drawing is about: whatever is pointed at, or whatever was picked. */
const focus = computed(() => props.hovered || props.selected);

const transform = computed(
  () => `translate(${props.offset.x}, ${props.offset.y}) scale(${props.zoom})`,
);

/** Both halves, so the template cannot ask for one and get the other. */
function rowsOf(name) {
  return rowsFor(props.tables, name);
}
</script>

<template>
  <svg
    class="absolute inset-0 h-full w-full"
    :class="panning ? 'cursor-grabbing' : 'cursor-grab'"
    :aria-label="`Diagram of ${title}`"
    role="img"
    @pointerdown="emit('pan-start', $event)"
    @pointermove="emit('pan-move', $event)"
    @pointerup="emit('pan-end')"
    @pointercancel="emit('pan-end')"
    @wheel="emit('zoom-wheel', $event)"
  >
    <defs>
      <!-- Crow's foot notation: many rows at the foreign key's end, exactly
           one at the key it points to. -->
      <marker
        v-for="lit in [false, true]"
        :id="lit ? 'fk-many-lit' : 'fk-many'"
        :key="`many-${lit}`"
        viewBox="-2 -8 18 16"
        refX="0"
        refY="0"
        markerWidth="18"
        markerHeight="16"
        markerUnits="userSpaceOnUse"
        orient="auto"
      >
        <path
          d="M0,-6 L11,0 L0,6 M0,0 L11,0 M14,-6 L14,6"
          fill="none"
          stroke-width="1.4"
          :class="lit ? 'stroke-accent' : 'stroke-fg-subtle'"
        />
      </marker>
      <marker
        v-for="lit in [false, true]"
        :id="lit ? 'fk-one-lit' : 'fk-one'"
        :key="`one-${lit}`"
        viewBox="-16 -8 18 16"
        refX="0"
        refY="0"
        markerWidth="18"
        markerHeight="16"
        markerUnits="userSpaceOnUse"
        orient="auto"
      >
        <path
          d="M-6,-6 L-6,6 M-11,-6 L-11,6"
          fill="none"
          stroke-width="1.4"
          :class="lit ? 'stroke-accent' : 'stroke-fg-subtle'"
        />
      </marker>
      <pattern
        id="grid"
        :width="24 * zoom"
        :height="24 * zoom"
        :x="offset.x"
        :y="offset.y"
        patternUnits="userSpaceOnUse"
      >
        <circle
          :cx="zoom"
          :cy="zoom"
          :r="Math.max(0.6, zoom)"
          class="fill-line"
        />
      </pattern>
    </defs>

    <!-- A dot grid, so panning reads as movement rather than as nothing. -->
    <rect width="100%" height="100%" fill="url(#grid)" />

    <g :transform="transform">
      <g v-if="caption" :transform="`translate(${caption.x}, ${caption.y})`">
        <text class="fill-fg-muted text-[13px] font-semibold">
          Tables without relationships
          <tspan class="fill-fg-subtle font-normal">
            · {{ caption.count }} with no foreign keys in or out
          </tspan>
        </text>
      </g>
      <path
        v-for="line in lines"
        :key="line.id"
        :d="roundedPath(line.points ?? [line.from, line.to])"
        fill="none"
        stroke-linejoin="round"
        :stroke-width="lineIsLit(line, focus) ? 2 : 1.4"
        :marker-start="
          lineIsLit(line, focus) ? 'url(#fk-many-lit)' : 'url(#fk-many)'
        "
        :marker-end="
          lineIsLit(line, focus) ? 'url(#fk-one-lit)' : 'url(#fk-one)'
        "
        :class="
          lineIsLit(line, focus)
            ? 'stroke-accent'
            : lineIsDimmed(line, focus)
              ? 'stroke-line opacity-40'
              : 'stroke-fg-subtle'
        "
      />
      <DiagramTableBox
        v-for="node in Object.values(nodes)"
        :key="node.id"
        :node="node"
        :rows="rowsOf(node.id)"
        :keyed="keyed"
        :expanded="expanded.has(node.id)"
        :selected="node.id === selected"
        :dimmed="tableIsDimmed(node.id, focus)"
        @hover="emit('hover', $event)"
        @select="emit('select', $event)"
        @drag-start="(id, event) => emit('drag-start', id, event)"
        @open-table="emit('open-table', $event)"
        @expand="emit('expand', $event)"
      />
    </g>
  </svg>
</template>

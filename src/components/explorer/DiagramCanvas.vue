<script setup>
/**
 * The drawing surface: the SVG, and the pan and zoom over it.
 *
 * Given nodes and lines, draws them. Knows nothing about databases, so the
 * part that can be wrong about an engine is nowhere near the part that can be
 * wrong about a gesture.
 */
import { computed } from "vue";
import { HEADER_HEIGHT, ROW_HEIGHT } from "../../lib/erLayout.js";
import {
  lineIsDimmed,
  lineIsLit,
  tableIsDimmed,
} from "../../lib/erHighlight.js";
import { rowsFor } from "../../lib/erSvg.js";

const props = defineProps({
  /** Positioned boxes, from erLayout. */
  nodes: { type: Object, required: true },
  lines: { type: Array, required: true },
  /** Trimmed tables, so a box knows its own rows. */
  tables: { type: Array, default: () => [] },
  /** Tables whose full column list should be drawn, by name. */
  expanded: { type: Set, default: () => new Set() },
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
    <g :transform="transform">
      <defs>
        <!-- Without an arrowhead a line does not say which end points at
             which table, which is the question every line is asked. -->
        <marker
          id="fk-arrow"
          viewBox="0 0 8 8"
          refX="7"
          refY="4"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path d="M0,0 L8,4 L0,8 z" class="fill-line-strong" />
        </marker>
        <marker
          id="fk-arrow-lit"
          viewBox="0 0 8 8"
          refX="7"
          refY="4"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path d="M0,0 L8,4 L0,8 z" class="fill-accent" />
        </marker>
        <pattern id="grid" width="24" height="24" patternUnits="userSpaceOnUse">
          <circle cx="1" cy="1" r="1" class="fill-line" />
        </pattern>
      </defs>

      <!-- A dot grid, so panning reads as movement rather than as nothing. -->
      <rect
        x="-20000"
        y="-20000"
        width="40000"
        height="40000"
        fill="url(#grid)"
      />

      <line
        v-for="line in lines"
        :key="line.id"
        :x1="line.from.x"
        :y1="line.from.y"
        :x2="line.to.x"
        :y2="line.to.y"
        :stroke-width="lineIsLit(line, focus) ? 2 : 1.25"
        :marker-end="
          lineIsLit(line, focus) ? 'url(#fk-arrow-lit)' : 'url(#fk-arrow)'
        "
        :class="
          lineIsDimmed(line, focus)
            ? 'stroke-line opacity-30'
            : 'stroke-line-strong'
        "
      />
      <g
        v-for="node in Object.values(nodes)"
        :key="node.id"
        :transform="`translate(${node.x}, ${node.y})`"
        class="cursor-pointer"
        :class="tableIsDimmed(node.id, focus) ? 'opacity-35' : ''"
        @pointerenter="emit('hover', node.id)"
        @pointerleave="emit('hover', null)"
        @click.stop="emit('select', node.id)"
      >
        <rect
          :width="node.width"
          :height="node.height"
          rx="6"
          :stroke-width="node.id === selected ? 2 : 1"
          :class="
            node.id === selected
              ? 'fill-surface stroke-accent'
              : 'fill-surface stroke-line'
          "
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
          v-for="(column, index) in rowsOf(node.id).columns"
          :key="column.name"
          class="cursor-pointer"
          @click.stop="emit('open-table', node.id)"
        >
          <text
            x="10"
            :y="HEADER_HEIGHT + index * ROW_HEIGHT + 13"
            class="text-[11px]"
            :class="column.primaryKey ? 'fill-fg font-bold' : 'fill-fg-muted'"
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
          v-if="rowsOf(node.id).hidden"
          x="10"
          :y="node.height - 4"
          class="fill-fg-subtle text-[10px] underline decoration-dotted"
          @click.stop="emit('expand', node.id)"
        >
          {{
            expanded.has(node.id)
              ? "show fewer"
              : `+${rowsOf(node.id).hidden} more`
          }}
        </text>
      </g>
    </g>
  </svg>
</template>

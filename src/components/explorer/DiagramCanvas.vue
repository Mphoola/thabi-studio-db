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
import { rowsFor } from "../../lib/erSvg.js";

const props = defineProps({
  /** Positioned boxes, from erLayout. */
  nodes: { type: Object, required: true },
  lines: { type: Array, required: true },
  /** Trimmed tables, so a box knows its own rows. */
  tables: { type: Array, default: () => [] },
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
]);

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
      <line
        v-for="line in lines"
        :key="line.id"
        :x1="line.from.x"
        :y1="line.from.y"
        :x2="line.to.x"
        :y2="line.to.y"
        class="stroke-line-strong"
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
          class="fill-fg-subtle text-[10px]"
        >
          +{{ rowsOf(node.id).hidden }} more
        </text>
      </g>
    </g>
  </svg>
</template>

<script setup>
/**
 * One table in the diagram: a header with its name and column count, then a
 * row per column with its key badge, name and type. Drawn inside the canvas's
 * SVG at the node's position.
 */
import { HEADER_HEIGHT, ROW_HEIGHT } from "../../lib/erLayout.js";

const props = defineProps({
  node: { type: Object, required: true },
  /** { columns, hidden } as rowsFor gives them. */
  rows: { type: Object, required: true },
  /** "table:column" of every foreign key column. */
  keyed: { type: Set, default: () => new Set() },
  expanded: { type: Boolean, default: false },
  selected: { type: Boolean, default: false },
  dimmed: { type: Boolean, default: false },
});
const emit = defineEmits([
  "hover",
  "select",
  "open-table",
  "expand",
  "drag-start",
]);

const rowY = (index) => HEADER_HEIGHT + index * ROW_HEIGHT;
const isForeign = (column) =>
  props.keyed.has(`${props.node.id}:${column.name}`);
const total = () => props.rows.columns.length + props.rows.hidden;
</script>

<template>
  <g
    :transform="`translate(${node.x}, ${node.y})`"
    class="cursor-move transition-opacity"
    :class="dimmed ? 'opacity-30' : ''"
    @pointerdown="emit('drag-start', node.id, $event)"
    @pointerenter="emit('hover', node.id)"
    @pointerleave="emit('hover', null)"
    @click.stop="emit('select', node.id)"
    @dblclick.stop="emit('open-table', node.id)"
  >
    <!-- A soft offset copy reads as a shadow without a filter. -->
    <rect
      x="0"
      y="2"
      :width="node.width"
      :height="node.height"
      rx="8"
      class="fill-line opacity-60"
    />
    <rect
      :width="node.width"
      :height="node.height"
      rx="8"
      :stroke-width="selected ? 2 : 1"
      :class="
        selected
          ? 'fill-surface stroke-accent'
          : 'fill-surface stroke-line-strong'
      "
    />
    <path
      :d="`M0.5,${HEADER_HEIGHT} V8 a7.5,7.5 0 0 1 7.5,-7.5 H${node.width - 8} a7.5,7.5 0 0 1 7.5,7.5 V${HEADER_HEIGHT} Z`"
      :class="selected ? 'fill-accent-subtle' : 'fill-surface-raised'"
    />
    <line
      x1="0"
      :y1="HEADER_HEIGHT"
      :x2="node.width"
      :y2="HEADER_HEIGHT"
      class="stroke-line"
    />
    <!-- Table glyph: a small grid. -->
    <g transform="translate(12, 11)" class="stroke-fg-subtle" fill="none">
      <rect width="12" height="12" rx="2" stroke-width="1.25" />
      <line x1="0" y1="4.5" x2="12" y2="4.5" stroke-width="1.25" />
      <line x1="5" y1="4.5" x2="5" y2="12" stroke-width="1.25" />
    </g>
    <text x="32" y="21.5" class="fill-fg text-[12.5px] font-semibold">
      {{ node.id }}
    </text>
    <text
      :x="node.width - 12"
      y="21.5"
      text-anchor="end"
      class="fill-fg-subtle text-[10.5px]"
    >
      {{ total() }}
    </text>

    <g
      v-for="(column, index) in rows.columns"
      :key="column.name"
      class="group/row"
    >
      <rect
        x="1"
        :y="rowY(index)"
        :width="node.width - 2"
        :height="ROW_HEIGHT"
        class="fill-transparent group-hover/row:fill-surface-hover"
      />
      <g v-if="column.primaryKey || isForeign(column)">
        <rect
          x="10"
          :y="rowY(index) + 5"
          width="20"
          height="12"
          rx="3"
          :class="
            column.primaryKey ? 'fill-warning-subtle' : 'fill-accent-subtle'
          "
        />
        <text
          x="20"
          :y="rowY(index) + 14"
          text-anchor="middle"
          class="text-[8px] font-bold tracking-wide"
          :class="column.primaryKey ? 'fill-warning-text' : 'fill-accent-text'"
        >
          {{ column.primaryKey ? "PK" : "FK" }}
        </text>
      </g>
      <text
        x="38"
        :y="rowY(index) + 15"
        class="text-[12px]"
        :class="column.primaryKey ? 'fill-fg font-semibold' : 'fill-fg'"
      >
        {{ column.name
        }}<tspan v-if="column.nullable" class="fill-fg-subtle">?</tspan>
      </text>
      <text
        :x="node.width - 12"
        :y="rowY(index) + 15"
        text-anchor="end"
        class="fill-fg-muted font-mono text-[11px]"
      >
        {{ column.dataType }}
      </text>
    </g>

    <text
      v-if="rows.hidden || expanded"
      x="38"
      :y="rowY(rows.columns.length) + 15"
      class="fill-accent-text text-[11px] hover:underline"
      @click.stop="emit('expand', node.id)"
    >
      {{ expanded ? "Show fewer" : `${rows.hidden} more columns…` }}
    </text>
  </g>
</template>

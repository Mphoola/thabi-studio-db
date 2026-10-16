<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  CHART_ROWS,
  chartData,
  numericColumns,
  ticks,
} from "../../lib/resultChart.js";
import SegmentedControl from "../common/SegmentedControl.vue";
import SelectInput from "../common/SelectInput.vue";

/**
 * A bar or line chart of a result: one column as labels, one numeric
 * column as values. Uses the first rows (up to CHART_ROWS); colours are the
 * tag tokens, so it follows light and dark mode.
 */
const props = defineProps({ source: { type: Object, required: true } });

const rows = ref(props.source.firstRows ?? []);
const loading = ref(false);
async function loadRows() {
  const want = Math.min(CHART_ROWS, props.source.total ?? rows.value.length);
  if (rows.value.length >= want || !props.source.load) return;
  loading.value = true;
  try {
    rows.value = await props.source.load(0, want, null);
  } finally {
    loading.value = false;
  }
}
onMounted(loadRows);

const columns = computed(() => props.source.columns);
const numeric = computed(() => numericColumns(columns.value, rows.value));
const xIndex = ref("0");
const yIndex = ref("");
const style = ref("bar");
watch(
  numeric,
  (list) => {
    if (!list.some((c) => String(c.index) === yIndex.value)) {
      // Prefer a measure (total, count) over a key column like `id`.
      const measure =
        list.find((c) => !/(^|_)id$/i.test(c.column.name)) ?? list[0];
      yIndex.value = measure ? String(measure.index) : "";
    }
    if (xIndex.value === yIndex.value && columns.value.length > 1)
      xIndex.value = String(
        columns.value.findIndex((_, i) => String(i) !== yIndex.value),
      );
  },
  { immediate: true },
);

const data = computed(() =>
  yIndex.value === ""
    ? null
    : chartData(columns.value, rows.value, Number(xIndex.value), [
        Number(yIndex.value),
      ]),
);

// Drawing area, measured so text keeps its size.
const box = ref(null);
const width = ref(600);
const height = 280;
const pad = { left: 56, right: 16, top: 12, bottom: 40 };
let observer;
onMounted(() => {
  observer = new ResizeObserver(
    ([entry]) => (width.value = Math.max(240, entry.contentRect.width)),
  );
  observer.observe(box.value);
});
onBeforeUnmount(() => observer?.disconnect());

const plot = computed(() => {
  const d = data.value;
  if (!d) return null;
  const innerW = width.value - pad.left - pad.right;
  const innerH = height - pad.top - pad.bottom;
  const n = Math.max(1, d.labels.length);
  const step = innerW / n;
  const y = (v) => pad.top + innerH - ((v - d.min) / (d.max - d.min)) * innerH;
  const x = (i) => pad.left + step * i + step / 2;
  const values = d.series[0].values;
  const bars = values.map((v, i) =>
    v === null
      ? null
      : {
          x: x(i) - Math.max(1, step * 0.35),
          w: Math.max(1, step * 0.7),
          y: Math.min(y(v), y(0)),
          h: Math.abs(y(v) - y(0)),
          label: `${d.labels[i]}: ${v.toLocaleString()}`,
        },
  );
  const line = values
    .map((v, i) => (v === null ? null : `${x(i)},${y(v)}`))
    .filter(Boolean)
    .join(" ");
  // Label at most ~12 x positions so they don't overlap.
  const every = Math.ceil(n / 12);
  const labels = d.labels
    .map((text, i) => ({
      text: text.length > 12 ? `${text.slice(0, 11)}…` : text,
      x: x(i),
      i,
    }))
    .filter((l) => l.i % every === 0);
  return {
    bars,
    line,
    labels,
    ticks: ticks(d.min, d.max).map((t) => ({ value: t, y: y(t) })),
    zero: y(0),
    colour: d.series[0].colour,
  };
});

const xOptions = computed(() =>
  columns.value.map((c, i) => ({ value: String(i), label: c.name })),
);
const yOptions = computed(() =>
  numeric.value.map(({ column, index }) => ({
    value: String(index),
    label: column.name,
  })),
);
// Whole class names, so Tailwind sees them (no string building).
const FILLS = {
  blue: "fill-tag-blue",
  orange: "fill-tag-orange",
  green: "fill-tag-green",
  purple: "fill-tag-purple",
  teal: "fill-tag-teal",
  red: "fill-tag-red",
};
const STROKES = {
  blue: "stroke-tag-blue",
  orange: "stroke-tag-orange",
  green: "stroke-tag-green",
  purple: "stroke-tag-purple",
  teal: "stroke-tag-teal",
  red: "stroke-tag-red",
};
const styles = [
  { value: "bar", label: "Bars" },
  { value: "line", label: "Line" },
];
</script>

<template>
  <div class="flex min-h-0 flex-col overflow-auto p-3">
    <div class="mb-2 flex flex-wrap items-end gap-2">
      <SelectInput
        v-model="xIndex"
        class="w-44"
        label="Labels"
        :options="xOptions"
      />
      <SelectInput
        v-model="yIndex"
        class="w-44"
        label="Values"
        :options="yOptions"
        :disabled="!yOptions.length"
      />
      <SegmentedControl v-model="style" :options="styles" label="Chart type" />
      <span class="text-xs text-fg-subtle">
        {{
          loading
            ? "Reading rows…"
            : `First ${rows.length.toLocaleString()} rows`
        }}
      </span>
    </div>
    <div ref="box" class="min-w-0">
      <p v-if="!plot" class="py-8 text-center text-sm text-fg-subtle">
        This result has no numeric column to chart.
      </p>
      <svg
        v-else
        :width="width"
        :height="height"
        role="img"
        :aria-label="`Chart of ${columns[Number(yIndex)]?.name} by ${columns[Number(xIndex)]?.name}`"
        class="text-fg-muted"
      >
        <g v-for="tick in plot.ticks" :key="tick.value">
          <line
            :x1="pad.left"
            :x2="width - pad.right"
            :y1="tick.y"
            :y2="tick.y"
            class="stroke-line"
          />
          <text
            :x="pad.left - 6"
            :y="tick.y + 4"
            text-anchor="end"
            class="fill-current text-xs"
          >
            {{ tick.value.toLocaleString() }}
          </text>
        </g>
        <template v-if="style === 'bar'">
          <rect
            v-for="(bar, i) in plot.bars.filter(Boolean)"
            :key="i"
            :x="bar.x"
            :y="bar.y"
            :width="bar.w"
            :height="Math.max(bar.h, 1)"
            rx="2"
            :class="FILLS[plot.colour]"
          >
            <title>{{ bar.label }}</title>
          </rect>
        </template>
        <polyline
          v-else
          :points="plot.line"
          fill="none"
          stroke-width="2"
          :class="STROKES[plot.colour]"
        />
        <line
          :x1="pad.left"
          :x2="width - pad.right"
          :y1="plot.zero"
          :y2="plot.zero"
          class="stroke-line-strong"
        />
        <text
          v-for="label in plot.labels"
          :key="label.i"
          :x="label.x"
          :y="height - pad.bottom + 16"
          text-anchor="middle"
          class="fill-current text-xs"
        >
          {{ label.text }}
        </text>
      </svg>
    </div>
  </div>
</template>

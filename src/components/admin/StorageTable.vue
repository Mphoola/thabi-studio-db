<script setup>
import { computed, ref } from "vue";
import { ArrowDown, ArrowUp } from "lucide-vue-next";
import { formatBytes } from "../../lib/formatSize.js";

/**
 * Tables by size, sortable, with each one's share of the total. Row counts
 * are the catalog's estimates: an exact count would read every table.
 */
const props = defineProps({
  tables: { type: Array, required: true },
});
const emit = defineEmits(["open-table"]);

const COLUMNS = [
  { key: "name", label: "Table" },
  { key: "rows", label: "Rows (est.)" },
  { key: "dataBytes", label: "Data" },
  { key: "indexBytes", label: "Indexes" },
  { key: "totalBytes", label: "Total" },
];
const sort = ref({ key: "totalBytes", descending: true });
const total = computed(() =>
  props.tables.reduce((sum, t) => sum + (t.totalBytes ?? 0), 0),
);
const sorted = computed(() => {
  const { key, descending } = sort.value;
  return [...props.tables].sort((a, b) => {
    const left = a[key] ?? -1;
    const right = b[key] ?? -1;
    const order =
      typeof left === "string"
        ? left.localeCompare(right)
        : Number(left) - Number(right);
    return descending ? -order : order;
  });
});

function sortBy(key) {
  sort.value =
    sort.value.key === key
      ? { key, descending: !sort.value.descending }
      : { key, descending: key !== "name" };
}
</script>

<template>
  <div class="min-h-0 flex-1 overflow-auto">
    <p class="px-4 py-2 text-sm text-fg-muted">
      {{ tables.length }} tables, {{ formatBytes(total) }} in all.
    </p>
    <table class="w-full text-sm">
      <thead
        class="sticky top-0 bg-grid-header text-left text-xs text-fg-muted"
      >
        <tr>
          <th
            v-for="column in COLUMNS"
            :key="column.key"
            class="px-3 py-1.5 font-semibold"
            :class="column.key !== 'name' && 'text-right'"
            :aria-sort="
              sort.key === column.key
                ? sort.descending
                  ? 'descending'
                  : 'ascending'
                : undefined
            "
          >
            <button
              type="button"
              class="inline-flex items-center gap-1 hover:text-fg"
              @click="sortBy(column.key)"
            >
              {{ column.label }}
              <component
                :is="sort.descending ? ArrowDown : ArrowUp"
                v-if="sort.key === column.key"
                :size="12"
                :stroke-width="1.75"
                aria-hidden="true"
              />
            </button>
          </th>
          <th class="w-40 px-3 py-1.5 font-semibold">Share</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="table in sorted"
          :key="`${table.schema}.${table.name}`"
          class="border-t border-line"
        >
          <td class="px-3 py-1.5">
            <button
              type="button"
              class="font-mono text-xs text-accent-text hover:underline"
              @click="emit('open-table', table)"
            >
              {{ table.name }}
            </button>
          </td>
          <td class="px-3 py-1.5 text-right tabular-nums">
            {{ table.rows === null ? "—" : table.rows.toLocaleString() }}
          </td>
          <td class="px-3 py-1.5 text-right tabular-nums">
            {{ formatBytes(table.dataBytes) }}
          </td>
          <td class="px-3 py-1.5 text-right tabular-nums">
            {{ formatBytes(table.indexBytes) }}
          </td>
          <td class="px-3 py-1.5 text-right font-medium tabular-nums">
            {{ formatBytes(table.totalBytes) }}
          </td>
          <td class="px-3 py-1.5">
            <div class="h-1.5 rounded-full bg-surface-hover" aria-hidden="true">
              <div
                class="h-1.5 rounded-full bg-accent"
                :style="{
                  width: `${total ? ((table.totalBytes ?? 0) / total) * 100 : 0}%`,
                }"
              />
            </div>
          </td>
        </tr>
        <tr v-if="!tables.length">
          <td colspan="6" class="px-3 py-4 text-center text-sm text-fg-subtle">
            No tables here.
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<script setup>
import { computed } from "vue";

/** Read-only table of the first rows of an object. */
const props = defineProps({ result: { type: Object, required: true } });

const NUMERIC =
  /^(int|integer|smallint|bigint|tinyint|mediumint|int[248]|decimal|numeric|real|float|float[48]|double|money|smallmoney|bit|i\d+|f\d+|intn|floatn|decimaln|numericn)/i;
const numeric = computed(() =>
  props.result.columns.map((column) => NUMERIC.test(column.typeName)),
);
const summary = computed(() => {
  const count = props.result.rows.length.toLocaleString();
  if (props.result.truncated) return `First ${count} rows`;
  return props.result.rows.length === 1 ? "1 row" : `${count} rows`;
});
</script>

<template>
  <div class="flex flex-col">
    <div class="min-h-0 flex-1 overflow-auto">
      <table
        v-if="result.columns.length"
        class="selectable min-w-full border-collapse font-mono text-sm"
      >
        <thead class="sticky top-0 z-10">
          <tr>
            <th
              class="w-10 border-r border-b border-line bg-grid-header px-2"
              aria-label="Row number"
            />
            <th
              v-for="column in result.columns"
              :key="column.name"
              class="h-7 border-r border-b border-line bg-grid-header px-2.5 text-left font-sans text-xs font-semibold whitespace-nowrap text-fg-muted"
            >
              {{ column.name }}
              <span class="ml-1 font-normal text-fg-subtle">
                {{ column.typeName.toLowerCase() }}
              </span>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(row, r) in result.rows"
            :key="r"
            class="even:bg-grid-row-alt"
          >
            <td
              class="border-r border-line px-2 text-right text-fg-disabled tabular-nums"
            >
              {{ r + 1 }}
            </td>
            <td
              v-for="(cell, c) in row"
              :key="c"
              class="h-6 max-w-96 truncate border-r border-line px-2.5"
              :class="[
                numeric[c] && 'text-right tabular-nums',
                cell.kind === 'null' && 'text-grid-null italic',
                cell.kind === 'binary' && 'text-fg-muted',
              ]"
              :title="cell.kind === 'value' ? cell.value : undefined"
            >
              <template v-if="cell.kind === 'null'">NULL</template>
              <template v-else-if="cell.kind === 'binary'">
                ‹{{ cell.value.bytes.toLocaleString() }} bytes›
              </template>
              <template v-else>{{ cell.value }}</template>
            </td>
          </tr>
        </tbody>
      </table>
      <p v-if="!result.rows.length" class="p-6 text-center text-fg-subtle">
        No rows.
      </p>
    </div>
    <footer
      class="flex h-6 shrink-0 items-center border-t border-line bg-toolbar px-3 text-xs text-fg-muted"
    >
      {{ summary }} · {{ result.elapsedMs.toLocaleString() }} ms · Read-only
      preview
    </footer>
  </div>
</template>

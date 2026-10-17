<script setup>
/**
 * The table list, with a filter.
 *
 * A hundred tables laid out by elk is far wider than any screen, so finding
 * one means either hunting across a zoomed-out drawing or scrolling a list.
 * The list is faster, and it gives the drawing somewhere to navigate to.
 */
import { computed, ref } from "vue";
import { Search, X } from "lucide-vue-next";
import { fuzzyScore } from "../../lib/fuzzy.js";
import EmptyState from "../common/EmptyState.vue";

const props = defineProps({
  tables: { type: Array, required: true },
  selected: { type: String, default: null },
  /** Column names are searchable too: people look for a column, not a table. */
  columnsByTable: { type: Object, default: () => ({}) },
});
const emit = defineEmits(["select"]);

const query = ref("");

/**
 * Matches a table's name, and names the column that matched when the hit was a
 * column — "which table has created_at" is as common a question as the other
 * way round.
 */
const matches = computed(() => {
  const needle = query.value.trim();
  if (!needle) return props.tables.map((table) => ({ table, via: null }));
  const found = [];
  for (const table of props.tables) {
    if (fuzzyScore(needle, table.name) > 0) {
      found.push({ table, via: null });
      continue;
    }
    const columns = props.columnsByTable[table.name] ?? [];
    const column = columns.find((name) => fuzzyScore(needle, name) > 0);
    if (column) found.push({ table, via: column });
  }
  return found;
});

function clear() {
  query.value = "";
}
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <div class="shrink-0 border-b border-line p-2">
      <div class="relative">
        <Search
          :size="14"
          :stroke-width="1.75"
          class="pointer-events-none absolute top-1/2 left-2 -translate-y-1/2 text-fg-subtle"
          aria-hidden="true"
        />
        <input
          v-model="query"
          type="search"
          placeholder="Find a table or column"
          aria-label="Find a table or column"
          class="w-full rounded border border-line bg-surface py-1 pr-6 pl-7 text-xs outline-none focus-visible:border-fg-subtle"
        />
        <button
          v-if="query"
          type="button"
          class="absolute top-1/2 right-1 -translate-y-1/2 text-fg-subtle hover:text-fg"
          aria-label="Clear"
          @click="clear"
        >
          <X :size="12" :stroke-width="1.75" aria-hidden="true" />
        </button>
      </div>
    </div>

    <ul class="min-h-0 flex-1 overflow-auto py-1">
      <li v-for="{ table, via } in matches" :key="table.name">
        <button
          type="button"
          class="flex w-full items-baseline gap-1.5 px-2 py-0.5 text-left text-xs hover:bg-surface-hover"
          :class="
            table.name === selected
              ? 'bg-surface-active text-fg'
              : 'text-fg-muted'
          "
          @click="emit('select', table.name)"
        >
          <span class="truncate">{{ table.name }}</span>
          <span
            v-if="via"
            class="ml-auto shrink-0 truncate text-[10px] text-fg-subtle"
          >
            via {{ via }}
          </span>
        </button>
      </li>
    </ul>

    <EmptyState
      v-if="!matches.length"
      :icon="Search"
      title="Nothing matches"
      class="p-3"
    >
      <template #message>No table or column called “{{ query }}”.</template>
    </EmptyState>
  </div>
</template>

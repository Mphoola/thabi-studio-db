<script setup>
import { KeyRound } from "lucide-vue-next";

/** Columns and indexes of a table or view. */
defineProps({
  structure: { type: Object, required: true },
  kind: { type: String, required: true },
});

const th =
  "h-7 border-b border-line bg-grid-header px-3 text-left text-xs font-semibold text-fg-muted";
const td = "h-7 border-b border-line px-3";
</script>

<template>
  <div class="selectable flex flex-col gap-6 overflow-auto p-4">
    <section class="flex flex-col gap-2">
      <h2 class="text-sm font-semibold">
        Columns
        <span class="font-normal text-fg-subtle">
          {{ structure.columns.length }}
        </span>
      </h2>
      <table
        class="w-full border-collapse rounded-md border border-line text-sm"
      >
        <thead>
          <tr>
            <th :class="th" class="w-10">#</th>
            <th :class="th">Name</th>
            <th :class="th">Type</th>
            <th :class="th">Nullable</th>
            <th :class="th">Default</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(column, index) in structure.columns" :key="column.name">
            <td :class="td" class="text-fg-subtle tabular-nums">
              {{ index + 1 }}
            </td>
            <td :class="td" class="font-medium">
              <span class="flex items-center gap-1.5">
                {{ column.name }}
                <KeyRound
                  v-if="column.primaryKey"
                  :size="12"
                  :stroke-width="1.75"
                  class="text-warning-text"
                  :aria-label="`Primary key, position ${column.primaryKey}`"
                />
              </span>
            </td>
            <td :class="td" class="font-mono text-fg-muted">
              {{ column.dataType }}
            </td>
            <td :class="td" class="text-fg-muted">
              {{ column.nullable ? "Yes" : "No" }}
            </td>
            <td :class="td" class="max-w-64 truncate font-mono text-fg-muted">
              <span v-if="column.default === null" class="text-fg-subtle"
                >—</span
              >
              <template v-else>{{ column.default }}</template>
            </td>
          </tr>
        </tbody>
      </table>
    </section>

    <section class="flex flex-col gap-2">
      <h2 class="text-sm font-semibold">
        Indexes
        <span v-if="structure.indexes" class="font-normal text-fg-subtle">
          {{ structure.indexes.length }}
        </span>
      </h2>
      <p v-if="!structure.indexes" class="text-sm text-fg-subtle">
        Indexes don’t apply to {{ kind === "view" ? "views" : "this object" }}.
      </p>
      <p v-else-if="!structure.indexes.length" class="text-sm text-fg-subtle">
        No indexes.
      </p>
      <table
        v-else
        class="w-full border-collapse rounded-md border border-line text-sm"
      >
        <thead>
          <tr>
            <th :class="th">Name</th>
            <th :class="th">Columns</th>
            <th :class="th">Type</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="index in structure.indexes" :key="index.name">
            <td :class="td" class="font-medium">{{ index.name }}</td>
            <td :class="td" class="font-mono text-fg-muted">
              {{
                index.columns.length ? index.columns.join(", ") : "expression"
              }}
            </td>
            <td :class="td" class="text-fg-muted">
              {{
                index.primary
                  ? "Primary key"
                  : index.unique
                    ? "Unique"
                    : "Index"
              }}
            </td>
          </tr>
        </tbody>
      </table>
    </section>
  </div>
</template>

<script setup>
import { computed } from "vue";
import CheckboxInput from "../common/CheckboxInput.vue";
import SelectInput from "../common/SelectInput.vue";
import TextInput from "../common/TextInput.vue";
import TypeInput from "../explorer/TypeInput.vue";

/**
 * Which file column fills which table column; or, for a new table, its name
 * and the columns to make, with types guessed from the file.
 */
const props = defineProps({
  state: { type: Object, required: true },
  /** Importing into a table that exists. */
  existing: { type: Boolean, default: true },
  types: { type: Array, default: () => [] },
});
const s = props.state;

const fileOptions = computed(() => [
  { value: -1, label: "— leave empty —" },
  ...(s.preview.value?.columns ?? []).map((name, i) => ({
    value: i,
    label: name,
  })),
]);
</script>

<template>
  <div class="flex flex-col gap-3">
    <template v-if="existing">
      <p class="text-sm text-fg-muted">
        Matched by name where the names agree. A column left empty takes its
        default.
      </p>
      <div
        class="grid max-h-80 grid-cols-[minmax(0,1fr)_16rem] items-center gap-x-4 gap-y-1.5 overflow-auto"
      >
        <template v-for="(column, i) in s.tableColumns.value" :key="column">
          <span class="truncate font-mono text-sm">{{ column }}</span>
          <SelectInput
            v-model="s.chosen.value[i]"
            :options="fileOptions"
            :aria-label="`File column for ${column}`"
          />
        </template>
      </div>
    </template>
    <template v-else>
      <TextInput
        v-model="s.newTable.name"
        label="New table name"
        autocomplete="off"
      />
      <p class="text-sm text-fg-muted">
        Types are guessed from the first rows; change any that are wrong. The
        table is made first, and stays if the import is stopped.
      </p>
      <div
        class="grid max-h-72 grid-cols-[auto_minmax(0,1fr)_12rem] items-center gap-x-3 gap-y-1.5 overflow-auto"
      >
        <template v-for="(column, i) in s.newTable.columns" :key="i">
          <CheckboxInput
            v-model="column.include"
            :label="`Include ${column.name}`"
            class="[&_label]:sr-only"
          />
          <TextInput
            v-model="column.name"
            :aria-label="`Name for file column ${i + 1}`"
          />
          <TypeInput
            v-model="column.type"
            :suggestions="types"
            :aria-label="`Type of ${column.name}`"
          />
        </template>
      </div>
    </template>
    <div class="flex flex-wrap gap-4 border-t border-line pt-3">
      <CheckboxInput
        v-model="s.options.emptyAsNull"
        label="Empty values are NULL"
      />
      <CheckboxInput
        v-model="s.options.skipBadRows"
        label="Skip rows the server refuses"
        description="Otherwise the first refused row stops the import and nothing is kept."
      />
    </div>
  </div>
</template>

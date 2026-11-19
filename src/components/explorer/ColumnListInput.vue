<script setup>
import { computed, ref } from "vue";
import { X } from "lucide-vue-next";
import SelectInput from "../common/SelectInput.vue";

/**
 * An ordered list of columns, for keys and indexes on more than one column.
 * The order is the key order, so columns are added one at a time rather
 * than ticked in a list that would hide it.
 */
const model = defineModel({ type: Array, default: () => [] });
const props = defineProps({
  /** Column names to choose from. */
  options: { type: Array, required: true },
  label: { type: String, default: "Columns" },
});

const next = ref("");
const remaining = computed(() =>
  props.options
    .filter((name) => !model.value.includes(name))
    .map((name) => ({ value: name, label: name })),
);

function add(name) {
  if (name) model.value = [...model.value, name];
  next.value = "";
}

function remove(name) {
  model.value = model.value.filter((one) => one !== name);
}
</script>

<template>
  <div class="flex flex-col gap-1">
    <span class="text-sm text-fg-muted">{{ label }}</span>
    <div class="flex flex-wrap items-center gap-1">
      <span
        v-for="(name, index) in model"
        :key="name"
        class="flex h-6 items-center gap-1 rounded-md border border-line bg-surface px-1.5 font-mono text-xs"
      >
        <span class="text-fg-subtle">{{ index + 1 }}</span>
        {{ name }}
        <button
          type="button"
          class="rounded text-fg-subtle hover:text-fg focus-visible:ring-3 focus-visible:ring-focus focus-visible:outline-none"
          :aria-label="`Remove ${name}`"
          @click="remove(name)"
        >
          <X :size="12" :stroke-width="1.75" aria-hidden="true" />
        </button>
      </span>
      <SelectInput
        v-if="remaining.length"
        v-model="next"
        class="w-36"
        :options="[
          { value: '', label: model.length ? 'Add column…' : 'Column…' },
          ...remaining,
        ]"
        :aria-label="`Add a column to ${label}`"
        @update:model-value="add"
      />
    </div>
  </div>
</template>

<script setup>
import { computed } from "vue";

/** A JSON value as a foldable tree (native <details>, so keyboard works). */
const props = defineProps({
  value: { type: null, required: true },
  name: { type: [String, Number], default: null },
  /** Nesting level: the first two levels start open. */
  depth: { type: Number, default: 0 },
});

const isContainer = computed(
  () => props.value !== null && typeof props.value === "object",
);
const entries = computed(() =>
  Array.isArray(props.value)
    ? props.value.map((v, i) => [i, v])
    : Object.entries(props.value ?? {}),
);
const summary = computed(() =>
  Array.isArray(props.value)
    ? `[${props.value.length}]`
    : `{${Object.keys(props.value ?? {}).length}}`,
);
const scalar = computed(() =>
  typeof props.value === "string"
    ? JSON.stringify(props.value)
    : String(props.value),
);
const scalarClass = computed(() => {
  if (props.value === null) return "text-grid-null italic";
  if (typeof props.value === "string") return "text-syntax-string";
  if (typeof props.value === "number") return "text-syntax-number";
  return "text-syntax-keyword";
});
</script>

<template>
  <details v-if="isContainer" :open="depth < 2" class="pl-4">
    <summary class="-ml-4 cursor-default select-none">
      <span v-if="name !== null" class="text-fg-muted">{{ name }}: </span>
      <span class="text-fg-subtle">{{ summary }}</span>
    </summary>
    <JsonTree
      v-for="[key, child] in entries"
      :key="key"
      :name="key"
      :value="child"
      :depth="depth + 1"
    />
  </details>
  <div v-else class="pl-4">
    <span v-if="name !== null" class="text-fg-muted">{{ name }}: </span>
    <span :class="scalarClass">{{ scalar }}</span>
  </div>
</template>

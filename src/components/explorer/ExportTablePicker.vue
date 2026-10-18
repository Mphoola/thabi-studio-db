<script setup>
/**
 * Which tables a dump contains.
 *
 * Its own component because it is a list with its own state and its own
 * keyboard behaviour, and the dialog around it is about what the file means
 * rather than about which tables are in it.
 *
 * Nothing selected means all of them, which is why the count says "N of M"
 * rather than showing a list with every box empty — an export that quietly
 * took nothing is worse than one that said so.
 */
import { computed } from "vue";

const props = defineProps({
  available: { type: Array, required: true },
  /** Explicitly chosen. Empty means every table. */
  picked: { type: Array, required: true },
});
const emit = defineEmits(["toggle", "all", "none"]);

const chosen = computed(() =>
  props.picked.length ? props.picked : [...props.available],
);
const isPicked = (name) =>
  props.picked.length === 0 || props.picked.includes(name);
</script>

<template>
  <section>
    <div class="mb-1.5 flex items-center gap-2">
      <h3 class="text-xs font-semibold text-fg-muted">Tables</h3>
      <button
        type="button"
        class="text-xs text-fg-muted hover:text-fg"
        @click="emit('all')"
      >
        All
      </button>
      <button
        type="button"
        class="text-xs text-fg-muted hover:text-fg"
        @click="emit('none')"
      >
        None
      </button>
      <span class="ml-auto text-xs text-fg-subtle">
        {{ chosen.length }} of {{ available.length }}
      </span>
    </div>
    <div class="max-h-40 overflow-auto rounded border border-line p-1">
      <label
        v-for="name in available"
        :key="name"
        class="flex items-center gap-2 rounded px-1 py-0.5 text-sm hover:bg-surface-hover"
      >
        <input
          type="checkbox"
          :checked="isPicked(name)"
          @change="emit('toggle', name)"
        />
        <span class="truncate font-mono text-xs">{{ name }}</span>
      </label>
    </div>
  </section>
</template>

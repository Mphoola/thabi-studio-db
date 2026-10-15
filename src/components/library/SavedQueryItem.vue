<script setup>
/** One saved query row in the Saved sidebar. */
defineProps({
  saved: { type: Object, required: true },
  indented: { type: Boolean, default: false },
});
defineEmits(["open", "menu"]);
</script>

<template>
  <button
    type="button"
    class="flex w-full flex-col gap-0.5 rounded-md py-1.5 pr-2 text-left hover:bg-surface-hover focus-visible:bg-surface-hover"
    :class="indented ? 'pl-7' : 'pl-2'"
    @click="$emit('open', saved)"
    @contextmenu.prevent="$emit('menu', $event, saved)"
  >
    <span class="truncate text-sm font-medium text-fg">{{ saved.name }}</span>
    <span class="truncate font-mono text-xs text-fg-subtle">
      {{ saved.notes || saved.queryText.split("\n")[0] }}
    </span>
  </button>
</template>

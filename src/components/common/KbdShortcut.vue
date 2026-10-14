<script setup>
import { computed } from "vue";
import { formatShortcut } from "../../lib/shortcuts.js";

const props = defineProps({ shortcut: { type: String, required: true } });
const text = computed(() => formatShortcut(props.shortcut));
</script>

<template>
  <kbd
    class="inline-flex h-4 items-center rounded-sm bg-surface-hover px-1 font-sans text-xs text-fg-subtle"
    >{{ text }}</kbd
  >
</template>

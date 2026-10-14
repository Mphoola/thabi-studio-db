<script setup>
import { useAppInfo } from "../../composables/useAppInfo.js";

const { info, error } = useAppInfo();
</script>

<template>
  <footer
    class="flex h-6 shrink-0 items-center justify-between border-t border-line bg-sidebar px-3 text-xs text-fg-subtle"
  >
    <span>No active connection</span>
    <span v-if="info">v{{ info.version }}</span>
    <span v-else-if="error" :title="error">Backend unavailable</span>
  </footer>
</template>

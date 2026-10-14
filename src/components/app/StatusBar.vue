<script setup>
import { useAppInfo } from "../../composables/useAppInfo.js";

const { info, error } = useAppInfo();
</script>

<template>
  <footer
    class="flex h-6 shrink-0 items-center gap-3 border-t border-line bg-toolbar px-3 text-xs text-fg-muted"
  >
    <span class="flex items-center gap-1.5">
      <span class="size-2 rounded-full bg-tag-gray" aria-hidden="true" />
      No connection
    </span>
    <span class="ml-auto" />
    <span v-if="info">v{{ info.version }}</span>
    <span v-else-if="error" :title="error">Backend unavailable</span>
  </footer>
</template>

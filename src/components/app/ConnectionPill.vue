<script setup>
import { vTooltip } from "../../composables/useTooltip.js";

// Phase 3 replaces this with the connection switcher (⇧⌘K). Until saved
// connections exist it only states that nothing is connected.
</script>

<template>
  <div
    v-tooltip="'Saved connections arrive in the next build'"
    class="flex h-6 items-center gap-2 rounded-md border border-line bg-surface px-2.5 text-sm text-fg-muted"
    role="status"
    tabindex="0"
  >
    <span class="size-2 rounded-full bg-tag-gray" aria-hidden="true" />
    No connection
  </div>
</template>

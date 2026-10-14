<script setup>
import BaseButton from "../common/BaseButton.vue";

defineEmits(["open-about"]);
</script>

<template>
  <aside
    class="flex w-60 shrink-0 flex-col border-r border-line bg-sidebar"
    aria-label="Connections"
  >
    <div class="flex h-10 items-center px-3">
      <h2 class="text-xs font-semibold tracking-wide text-fg-subtle uppercase">
        Connections
      </h2>
    </div>
    <div class="flex-1 overflow-auto px-3 py-2">
      <p class="text-sm text-fg-subtle">No saved connections.</p>
    </div>
    <div class="border-t border-line p-2">
      <BaseButton
        variant="ghost"
        size="sm"
        class="w-full justify-start"
        @click="$emit('open-about')"
      >
        About Thabi Studio
      </BaseButton>
    </div>
  </aside>
</template>

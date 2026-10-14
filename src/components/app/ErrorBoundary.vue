<script setup>
import { onErrorCaptured, ref } from "vue";
import { toUserMessage } from "../../lib/errors.js";
import BaseButton from "../common/BaseButton.vue";

/**
 * Catches render and lifecycle errors in its subtree and shows a recovery
 * view instead of a blank window.
 */
const error = ref(null);

onErrorCaptured((err, _instance, info) => {
  console.error(`[ErrorBoundary] ${info}`, err);
  error.value = err;
  return false;
});

function reload() {
  window.location.reload();
}
</script>

<template>
  <div
    v-if="error"
    role="alert"
    class="flex h-screen flex-col items-center justify-center gap-3 bg-canvas p-8 text-center"
  >
    <h1 class="text-xl font-semibold">Thabi Studio hit an unexpected error</h1>
    <p class="selectable max-w-md text-fg-muted">{{ toUserMessage(error) }}</p>
    <div class="mt-2 flex gap-2">
      <BaseButton @click="error = null">Try again</BaseButton>
      <BaseButton variant="primary" @click="reload">Reload window</BaseButton>
    </div>
  </div>
  <slot v-else />
</template>

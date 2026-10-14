<script setup>
import { onMounted, ref, watch } from "vue";
import { useToasts } from "../../composables/useToasts.js";

const { toasts, dismiss } = useToasts();
const host = ref(null);

// A manual popover lives in the top layer, so toasts stay visible and
// clickable above modal dialogs. Re-showing it moves it back on top.
onMounted(() => host.value.showPopover());
watch(
  () => toasts.value.length,
  (count, previous) => {
    if (count > previous) {
      host.value.hidePopover();
      host.value.showPopover();
    }
  },
);

const toneClass = {
  info: "border-l-accent",
  success: "border-l-success",
  error: "border-l-danger",
};
</script>

<template>
  <div
    ref="host"
    popover="manual"
    class="pointer-events-none fixed top-auto right-4 bottom-10 left-auto m-0 flex w-80 flex-col gap-2 overflow-visible bg-transparent p-0"
    aria-live="polite"
    aria-relevant="additions"
  >
    <TransitionGroup
      enter-from-class="translate-y-2 opacity-0"
      leave-to-class="opacity-0"
      enter-active-class="transition duration-150 ease-out"
      leave-active-class="transition duration-100 ease-in"
    >
      <div
        v-for="toast in toasts"
        :key="toast.id"
        :role="toast.tone === 'error' ? 'alert' : 'status'"
        class="pointer-events-auto flex items-start gap-3 rounded-md border border-l-4 border-line bg-surface-raised px-3 py-2.5 shadow-popover"
        :class="toneClass[toast.tone]"
      >
        <div class="min-w-0 flex-1">
          <p class="font-medium">{{ toast.title }}</p>
          <p
            v-if="toast.detail"
            class="selectable mt-0.5 text-sm break-words text-fg-muted"
          >
            {{ toast.detail }}
          </p>
        </div>
        <button
          type="button"
          class="rounded-sm px-1 text-fg-subtle hover:text-fg"
          aria-label="Dismiss notification"
          @click="dismiss(toast.id)"
        >
          ✕
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>

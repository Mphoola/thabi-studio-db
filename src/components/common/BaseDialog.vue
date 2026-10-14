<script setup>
import { nextTick, onMounted, ref, useId, watch } from "vue";

/**
 * Modal dialog built on the native <dialog> element, which gives us focus
 * trapping, Escape handling and correct accessibility semantics for free.
 */
const props = defineProps({
  title: { type: String, required: true },
  description: { type: String, default: "" },
  size: {
    type: String,
    default: "md",
    validator: (v) => ["sm", "md", "lg"].includes(v),
  },
});

const open = defineModel("open", { type: Boolean, default: false });

const dialog = ref(null);
const titleId = useId();
const descriptionId = useId();
const widths = { sm: "w-[360px]", md: "w-[480px]", lg: "w-[640px]" };

async function sync() {
  await nextTick();
  const el = dialog.value;
  if (!el) return;
  if (open.value && !el.open) el.showModal();
  if (!open.value && el.open) el.close();
}

function onBackdropClick(event) {
  if (event.target === dialog.value) open.value = false;
}

watch(open, sync);
onMounted(sync);
</script>

<template>
  <dialog
    ref="dialog"
    :aria-labelledby="titleId"
    :aria-describedby="props.description ? descriptionId : undefined"
    class="m-auto max-w-[calc(100vw-48px)] rounded-lg bg-surface-raised p-0 text-fg shadow-dialog backdrop:bg-overlay"
    :class="widths[props.size]"
    @close="open = false"
    @click="onBackdropClick"
  >
    <div class="flex flex-col gap-4 p-5">
      <header class="flex flex-col gap-1">
        <h2 :id="titleId" class="text-lg font-semibold">{{ props.title }}</h2>
        <p v-if="props.description" :id="descriptionId" class="text-fg-muted">
          {{ props.description }}
        </p>
      </header>
      <div v-if="$slots.default">
        <slot />
      </div>
      <footer v-if="$slots.footer" class="flex justify-end gap-2">
        <slot name="footer" />
      </footer>
    </div>
  </dialog>
</template>

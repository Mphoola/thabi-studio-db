<script setup>
import { computed, ref } from "vue";
import { Copy } from "lucide-vue-next";
import { copyText } from "../../lib/clipboard.js";
import { useToasts } from "../../composables/useToasts.js";
import { inspectText } from "../../lib/gridText.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";

/** Shows one cell's full value: long text, pretty JSON, binary details. */
const open = ref(false);
const column = ref(null);
const cell = ref(null);
const toasts = useToasts();

const text = computed(() => inspectText(cell.value, column.value?.typeName));
const description = computed(() =>
  column.value ? `${column.value.typeName.toLowerCase()}` : "",
);

async function copy() {
  try {
    await copyText(text.value ?? "");
    toasts.info("Copied value");
  } catch {
    toasts.error("Couldn’t copy", "The clipboard isn’t available.");
  }
}

defineExpose({
  open(col, value) {
    column.value = col;
    cell.value = value;
    open.value = true;
  },
});
</script>

<template>
  <BaseDialog
    v-if="column"
    v-model:open="open"
    size="lg"
    :title="column.name"
    :description="description"
  >
    <p v-if="text === null" class="font-mono text-grid-null italic">NULL</p>
    <pre
      v-else
      class="selectable max-h-[55vh] overflow-auto rounded-md bg-surface-hover p-3 font-mono text-sm break-words whitespace-pre-wrap text-fg"
      >{{ text }}</pre>
    <p v-if="cell?.kind === 'binary'" class="mt-2 text-sm text-fg-subtle">
      Only the first bytes are shown. Copying or exporting the result includes
      every byte as hex.
    </p>
    <template #footer>
      <BaseButton v-if="text !== null" :icon="Copy" @click="copy"
        >Copy Value</BaseButton
      >
      <BaseButton variant="primary" @click="open = false">Done</BaseButton>
    </template>
  </BaseDialog>
</template>

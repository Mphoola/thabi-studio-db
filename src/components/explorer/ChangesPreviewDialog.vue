<script setup>
import { computed } from "vue";
import { useToasts } from "../../composables/useToasts.js";
import { copyText } from "../../lib/clipboard.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";

/** The exact statements staged edits will run, in order, before applying. */
const props = defineProps({
  statements: { type: Array, default: null },
  applying: { type: Boolean, default: false },
});
const emit = defineEmits(["close", "apply"]);
const toasts = useToasts();
const open = computed({
  get: () => Boolean(props.statements),
  set: (value) => !value && emit("close"),
});
const sql = computed(() =>
  (props.statements ?? []).map((s) => `${s};`).join("\n"),
);

async function copy() {
  await copyText(sql.value);
  toasts.info("Copied SQL");
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    size="lg"
    title="Review Changes"
    description="These statements run in one transaction. If any of them doesn’t change exactly one row, nothing is saved."
  >
    <pre
      class="selectable max-h-[50vh] overflow-auto rounded-md border border-line bg-canvas p-3 font-mono text-sm whitespace-pre-wrap text-fg"
      >{{ sql }}</pre>
    <template #footer>
      <BaseButton class="mr-auto" @click="copy">Copy SQL</BaseButton>
      <BaseButton @click="open = false">Close</BaseButton>
      <BaseButton variant="primary" :loading="applying" @click="emit('apply')">
        Apply Changes
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<script setup>
import { computed } from "vue";
import { formatShortcut } from "../../lib/shortcuts.js";
import BaseButton from "../common/BaseButton.vue";

/** Staged edits waiting to be applied: what changed, review, discard, apply. */
const props = defineProps({
  summary: { type: Object, required: true },
  applying: { type: Boolean, default: false },
});
defineEmits(["preview", "discard", "apply"]);

const text = computed(() => {
  const { edited, deleted, inserted } = props.summary;
  const parts = [];
  if (edited) parts.push(`${edited} edited`);
  if (deleted) parts.push(`${deleted} to delete`);
  if (inserted) parts.push(`${inserted} new`);
  return `${parts.join(", ")} · not saved yet`;
});
</script>

<template>
  <div
    class="flex h-9 shrink-0 items-center gap-2 border-b border-line bg-warning-subtle px-4 text-sm text-warning-text"
    role="status"
  >
    <span class="font-medium">{{ text }}</span>
    <BaseButton size="sm" class="ml-auto" @click="$emit('discard')">
      Discard
    </BaseButton>
    <BaseButton size="sm" @click="$emit('preview')">Review SQL…</BaseButton>
    <BaseButton
      size="sm"
      variant="primary"
      :loading="applying"
      @click="$emit('apply')"
    >
      Apply {{ formatShortcut("Mod+S") }}
    </BaseButton>
  </div>
</template>

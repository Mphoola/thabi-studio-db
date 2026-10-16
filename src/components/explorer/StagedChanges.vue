<script setup>
import { X } from "lucide-vue-next";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";

/**
 * What is staged, and how to take any of it back.
 *
 * Each change is listed by what it does rather than by the SQL it produces,
 * so a change can be undone before anything is run. Undoing is per change: a
 * staged list where undoing means throwing away four other edits is not much of
 * a review.
 */
defineProps({
  edits: { type: Array, required: true },
  /** How each change reads, from lib/structurePlan.js. */
  describe: { type: Function, required: true },
});
const emit = defineEmits(["undo", "discard"]);
</script>

<template>
  <div class="flex items-start gap-3 border-b border-line px-3 py-2">
    <ul class="flex min-w-0 flex-1 flex-col gap-1">
      <li
        v-for="(one, index) in edits"
        :key="`${describe(one)}:${index}`"
        class="flex items-center gap-2 text-xs text-fg-muted"
      >
        <span class="min-w-0 truncate">{{ describe(one) }}</span>
        <IconButton
          :icon="X"
          size="sm"
          label="Undo this change"
          @click="emit('undo', one)"
        />
      </li>
    </ul>
    <BaseButton size="sm" variant="ghost" @click="emit('discard')"
      >Discard</BaseButton
    >
  </div>
</template>

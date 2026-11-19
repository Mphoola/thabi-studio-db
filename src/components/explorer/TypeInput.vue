<script setup>
import { useId } from "vue";
import { borderClass, fieldClass } from "../common/fieldClasses.js";

/**
 * A column type: typed, with the engine's common types suggested. A fixed
 * list can't hold `VARCHAR(50)`, a domain or an extension's type.
 */
const model = defineModel({ type: String, default: "" });
defineProps({
  suggestions: { type: Array, default: () => [] },
  label: { type: String, default: "" },
  ariaLabel: { type: String, default: "" },
});
const id = useId();
</script>

<template>
  <div class="flex flex-col gap-1">
    <label v-if="label" :for="id" class="text-sm text-fg-muted">{{
      label
    }}</label>
    <input
      :id="id"
      v-model="model"
      :list="`${id}-types`"
      :class="[fieldClass, borderClass(''), 'font-mono text-sm']"
      spellcheck="false"
      autocomplete="off"
      :aria-label="ariaLabel || undefined"
    />
    <datalist :id="`${id}-types`">
      <option v-for="type in suggestions" :key="type" :value="type" />
    </datalist>
  </div>
</template>

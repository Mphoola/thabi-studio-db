<script setup>
import { useId } from "vue";

defineProps({
  label: { type: String, required: true },
  description: { type: String, default: "" },
  disabled: { type: Boolean, default: false },
  /** Keep the label for screen readers only, when a row already shows it. */
  hideLabel: { type: Boolean, default: false },
});

const model = defineModel({ type: Boolean, default: false });
const id = useId();
</script>

<template>
  <div class="flex items-start justify-between gap-4">
    <div class="flex flex-col" :class="hideLabel && 'sr-only'">
      <label :id="`${id}-label`" :for="id" class="text-base text-fg">
        {{ label }}
      </label>
      <p v-if="description" :id="`${id}-desc`" class="text-sm text-fg-subtle">
        {{ description }}
      </p>
    </div>
    <button
      :id="id"
      type="button"
      role="switch"
      :aria-checked="model"
      :aria-labelledby="`${id}-label`"
      :aria-describedby="description ? `${id}-desc` : undefined"
      :disabled="disabled"
      class="relative inline-flex h-[18px] w-8 shrink-0 items-center rounded-full transition-colors disabled:opacity-50"
      :class="model ? 'bg-accent' : 'bg-line-strong'"
      @click="model = !model"
    >
      <span
        class="size-3.5 rounded-full bg-knob shadow-popover transition-transform"
        :class="model ? 'translate-x-4' : 'translate-x-0.5'"
      />
    </button>
  </div>
</template>

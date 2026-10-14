<script setup>
import { vTooltip } from "../../composables/useTooltip.js";

/** Square icon-only button. `label` is required for screen readers and the tooltip. */
const props = defineProps({
  icon: { type: [Object, Function], required: true },
  label: { type: String, required: true },
  shortcut: { type: String, default: "" },
  size: {
    type: String,
    default: "md",
    validator: (v) => ["sm", "md"].includes(v),
  },
  pressed: { type: Boolean, default: undefined },
  disabled: { type: Boolean, default: false },
});
</script>

<template>
  <button
    v-tooltip="{ text: props.label, shortcut: props.shortcut }"
    type="button"
    :aria-label="props.label"
    :aria-pressed="props.pressed"
    :disabled="props.disabled"
    class="inline-flex items-center justify-center rounded-md text-fg-muted transition-colors hover:bg-surface-hover hover:text-fg active:bg-surface-active disabled:pointer-events-none disabled:opacity-40 aria-pressed:bg-surface-active aria-pressed:text-fg"
    :class="props.size === 'sm' ? 'size-6' : 'size-7'"
  >
    <component
      :is="props.icon"
      :size="props.size === 'sm' ? 14 : 16"
      :stroke-width="1.75"
      aria-hidden="true"
    />
  </button>
</template>

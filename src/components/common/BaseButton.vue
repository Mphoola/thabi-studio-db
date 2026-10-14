<script setup>
import { computed } from "vue";
import SpinnerIcon from "./SpinnerIcon.vue";

const props = defineProps({
  variant: {
    type: String,
    default: "secondary",
    validator: (v) => ["primary", "secondary", "ghost", "danger"].includes(v),
  },
  size: {
    type: String,
    default: "md",
    validator: (v) => ["sm", "md"].includes(v),
  },
  type: { type: String, default: "button" },
  /** A lucide icon component shown before the label. */
  icon: { type: [Object, Function], default: null },
  disabled: { type: Boolean, default: false },
  loading: { type: Boolean, default: false },
});

const variantClass = computed(
  () =>
    ({
      primary:
        "bg-accent text-accent-fg hover:bg-accent-hover active:bg-accent-active",
      secondary:
        "border border-line-strong bg-surface text-fg hover:bg-surface-hover active:bg-surface-active",
      ghost:
        "text-fg-muted hover:bg-surface-hover hover:text-fg active:bg-surface-active",
      danger: "bg-danger text-danger-fg hover:bg-danger-hover",
    })[props.variant],
);

const sizeClass = computed(() =>
  props.size === "sm" ? "h-6 px-2 text-sm" : "h-7 px-3 text-base",
);
</script>

<template>
  <button
    :type="type"
    :disabled="disabled || loading"
    :aria-busy="loading || undefined"
    class="inline-flex items-center justify-center gap-1.5 rounded-md font-medium whitespace-nowrap transition-colors disabled:pointer-events-none disabled:opacity-50"
    :class="[variantClass, sizeClass]"
  >
    <SpinnerIcon v-if="loading" :size="14" />
    <component
      :is="icon"
      v-else-if="icon"
      :size="14"
      :stroke-width="1.75"
      aria-hidden="true"
    />
    <slot />
  </button>
</template>

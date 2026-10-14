<script setup>
import { computed } from "vue";
import { CircleAlert, CircleCheck, Info, TriangleAlert } from "lucide-vue-next";

/** Inline message for error, warning, success and info states. */
const props = defineProps({
  tone: {
    type: String,
    default: "info",
    validator: (v) => ["info", "success", "warning", "danger"].includes(v),
  },
  title: { type: String, required: true },
  /** Detail such as a driver message. Selectable so it can be copied. */
  message: { type: String, default: "" },
  mono: { type: Boolean, default: false },
});

const style = computed(
  () =>
    ({
      info: { box: "bg-accent-subtle", text: "text-accent-text", icon: Info },
      success: {
        box: "bg-success-subtle",
        text: "text-success-text",
        icon: CircleCheck,
      },
      warning: {
        box: "bg-warning-subtle",
        text: "text-warning-text",
        icon: TriangleAlert,
      },
      danger: {
        box: "bg-danger-subtle",
        text: "text-danger-text",
        icon: CircleAlert,
      },
    })[props.tone],
);
</script>

<template>
  <div
    :role="tone === 'danger' ? 'alert' : 'status'"
    class="flex gap-2.5 rounded-md px-3 py-2.5"
    :class="style.box"
  >
    <component
      :is="style.icon"
      :size="16"
      :stroke-width="1.75"
      class="mt-px shrink-0"
      :class="style.text"
      aria-hidden="true"
    />
    <div class="flex min-w-0 flex-1 flex-col gap-1">
      <p class="font-medium" :class="style.text">{{ title }}</p>
      <p
        v-if="message"
        class="selectable break-words text-fg"
        :class="mono ? 'font-mono text-sm' : 'text-base'"
      >
        {{ message }}
      </p>
      <div v-if="$slots.default" class="mt-1 flex gap-2">
        <slot />
      </div>
    </div>
  </div>
</template>

<script setup>
/**
 * A form row with its label on the left, like a native settings form:
 * label column, then the control, then any hint or error under it.
 */
defineProps({
  label: { type: String, required: true },
  /** id of the control the label names, when there is a single one. */
  for: { type: String, default: "" },
  hint: { type: String, default: "" },
  error: { type: String, default: "" },
});
</script>

<template>
  <div class="grid grid-cols-[7.5rem_minmax(0,1fr)] items-start gap-x-3">
    <label
      :for="$props.for || undefined"
      class="pt-1 text-right text-sm text-fg-muted"
      >{{ label }}</label
    >
    <div class="flex min-w-0 flex-col gap-1">
      <slot />
      <p v-if="error" class="text-sm text-danger-text" role="alert">
        {{ error }}
      </p>
      <p v-else-if="hint" class="text-xs text-fg-subtle">{{ hint }}</p>
    </div>
  </div>
</template>

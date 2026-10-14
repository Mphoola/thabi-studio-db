<script setup>
/** Label, hint and error around a single control. Used by the inputs below. */
defineProps({
  id: { type: String, required: true },
  label: { type: String, default: "" },
  hint: { type: String, default: "" },
  error: { type: String, default: "" },
});
</script>

<template>
  <div class="flex flex-col gap-1">
    <label v-if="label" :for="id" class="text-sm font-medium text-fg">
      {{ label }}
    </label>
    <slot />
    <p v-if="error" :id="`${id}-error`" class="text-sm text-danger-text">
      {{ error }}
    </p>
    <p v-else-if="hint" :id="`${id}-hint`" class="text-sm text-fg-subtle">
      {{ hint }}
    </p>
  </div>
</template>

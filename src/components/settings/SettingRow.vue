<script setup>
/** One row in Settings: label and explanation on the left, control on the right. */
defineProps({
  title: { type: String, required: true },
  description: { type: String, default: "" },
});
</script>

<template>
  <div class="flex items-start justify-between gap-6 py-3 first:pt-0 last:pb-0">
    <div class="flex max-w-72 flex-col">
      <span class="text-base text-fg">{{ title }}</span>
      <span v-if="description" class="text-sm text-fg-subtle">{{
        description
      }}</span>
    </div>
    <slot />
  </div>
</template>

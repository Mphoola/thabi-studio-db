<script setup>
/** A titled card of related setting rows. */
defineProps({
  title: { type: String, default: "" },
  description: { type: String, default: "" },
});
</script>

<template>
  <section class="flex flex-col gap-2">
    <header v-if="title" class="flex flex-col px-1">
      <h3 class="text-sm font-semibold text-fg-muted">{{ title }}</h3>
      <p v-if="description" class="text-sm text-fg-subtle">
        {{ description }}
      </p>
    </header>
    <div
      class="flex flex-col divide-y divide-line rounded-lg border border-line bg-surface px-4"
    >
      <slot />
    </div>
  </section>
</template>

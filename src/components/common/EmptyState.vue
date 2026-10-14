<script setup>
/** Centered message for a view with nothing in it yet. */
defineProps({
  icon: { type: [Object, Function], default: null },
  title: { type: String, required: true },
  description: { type: String, default: "" },
  size: {
    type: String,
    default: "md",
    validator: (v) => ["sm", "md"].includes(v),
  },
});
</script>

<template>
  <div
    class="flex flex-col items-center justify-center text-center"
    :class="size === 'sm' ? 'gap-1.5 px-4 py-8' : 'gap-3 p-8'"
  >
    <span
      v-if="icon"
      class="flex items-center justify-center rounded-lg text-fg-subtle"
      :class="size === 'sm' ? 'mb-1' : 'mb-1 size-12 bg-surface-hover'"
    >
      <component
        :is="icon"
        :size="size === 'sm' ? 20 : 24"
        :stroke-width="1.5"
        aria-hidden="true"
      />
    </span>
    <h2
      class="font-semibold text-fg"
      :class="size === 'sm' ? 'text-base' : 'text-xl'"
    >
      {{ title }}
    </h2>
    <p
      v-if="description"
      class="max-w-sm text-fg-muted"
      :class="size === 'sm' ? 'text-sm' : 'text-base'"
    >
      {{ description }}
    </p>
    <div v-if="$slots.default" class="mt-2 flex flex-wrap justify-center gap-2">
      <slot />
    </div>
  </div>
</template>

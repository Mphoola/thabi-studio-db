<script setup>
defineProps({
  size: { type: Number, default: 16 },
  label: { type: String, default: "Loading" },
});
</script>

<template>
  <span
    role="status"
    :aria-label="label"
    class="inline-block shrink-0 animate-spin rounded-full border-2 border-current border-r-transparent opacity-70 motion-reduce:animate-[spin_1.5s_linear_infinite]"
    :style="{ width: `${size}px`, height: `${size}px` }"
  />
</template>

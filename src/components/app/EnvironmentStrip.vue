<script setup>
import { computed } from "vue";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";

// A thin line under the title bar in the connection's tag colour. Only
// Production and Staging show it, so it never becomes wallpaper.
const { selectedProfile } = useConnectionProfiles();
const strip = computed(
  () =>
    ({
      production: { class: "bg-tag-red", label: "Production connection" },
      staging: { class: "bg-tag-orange", label: "Staging connection" },
    })[selectedProfile.value?.environment] ?? null,
);
</script>

<template>
  <div
    v-if="strip"
    class="h-0.5 shrink-0"
    :class="strip.class"
    role="note"
    :aria-label="strip.label"
  />
</template>

<script setup>
import { computed } from "vue";
import { vTooltip } from "../../composables/useTooltip.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { environmentFor } from "../../lib/environments.js";

const store = useConnectionProfiles();
const { selectedProfile } = store;
const environment = computed(() =>
  selectedProfile.value
    ? environmentFor(selectedProfile.value.environment)
    : null,
);
const connected = computed(
  () => selectedProfile.value && store.isConnected(selectedProfile.value.id),
);
</script>

<template>
  <div
    v-tooltip="
      selectedProfile
        ? connected
          ? 'This profile is connected'
          : 'This profile is selected but not connected'
        : 'No connection profile is selected'
    "
    class="flex h-6 items-center gap-2 rounded-md border border-line bg-surface px-2.5 text-sm text-fg-muted"
    role="status"
    tabindex="0"
  >
    <span
      class="size-2 rounded-full"
      :class="environment?.dotClass ?? 'bg-tag-gray'"
      aria-hidden="true"
    />
    <template v-if="selectedProfile">
      <span class="max-w-36 truncate">{{ selectedProfile.name }}</span>
      <span :class="connected ? 'text-success-text' : 'text-fg-subtle'">
        {{ connected ? "Connected" : "Not connected" }}
      </span>
    </template>
    <span v-else>No connection</span>
  </div>
</template>

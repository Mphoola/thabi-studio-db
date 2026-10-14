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
const status = computed(() =>
  selectedProfile.value ? store.statusFor(selectedProfile.value.id) : null,
);
const connected = computed(() => status.value?.state === "connected");
const tooltip = computed(() => {
  if (!selectedProfile.value) return "No connection selected";
  if (connected.value) return `Connected to ${status.value.server.version}`;
  if (status.value?.state === "connecting") return "Connecting…";
  return "Not connected";
});
</script>

<template>
  <div
    v-tooltip="tooltip"
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

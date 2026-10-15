<script setup>
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { environmentFor } from "../../lib/environments.js";
import { useAppInfo } from "../../composables/useAppInfo.js";
import SecurityIndicator from "./SecurityIndicator.vue";

const { info, error } = useAppInfo();
const store = useConnectionProfiles();
const { selectedProfile } = store;
</script>

<template>
  <footer
    class="flex h-6 shrink-0 items-center gap-3 border-t border-line bg-toolbar px-3 text-xs text-fg-muted"
  >
    <span class="flex items-center gap-1.5">
      <span
        class="size-2 rounded-full"
        :class="
          selectedProfile
            ? environmentFor(selectedProfile.environment).dotClass
            : 'bg-tag-gray'
        "
        aria-hidden="true"
      />
      <template v-if="selectedProfile">
        <b class="font-medium text-fg">{{ selectedProfile.name }}</b>
        <span v-if="store.isConnected(selectedProfile.id)">
          {{ store.statusFor(selectedProfile.id).server.version }} ·
          {{ selectedProfile.username }}@{{ selectedProfile.host }}:{{
            selectedProfile.port
          }}
        </span>
        <span v-else>Not connected</span>
      </template>
      <template v-else>No connection</template>
    </span>
    <SecurityIndicator v-if="selectedProfile" :profile="selectedProfile" />
    <span class="ml-auto" />
    <span v-if="info">v{{ info.version }}</span>
    <span v-else-if="error" :title="error">Backend unavailable</span>
  </footer>
</template>

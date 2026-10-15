<script setup>
import { Database, Plus } from "lucide-vue-next";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { runCommand } from "../../composables/useCommands.js";
import BaseButton from "../common/BaseButton.vue";
import EmptyState from "../common/EmptyState.vue";
import ConnectionsView from "../connections/ConnectionsView.vue";

const { profiles, loaded, nativeAvailable } = useConnectionProfiles();
</script>

<template>
  <ConnectionsView v-if="profiles.length" />
  <div
    v-else-if="!loaded && nativeAvailable"
    class="flex h-full items-center justify-center text-sm text-fg-subtle"
    role="status"
  >
    Loading saved connections…
  </div>
  <section v-else class="flex h-full flex-col items-center justify-center">
    <EmptyState
      :icon="Database"
      title="Connect to your first database"
      :description="
        nativeAvailable
          ? 'Thabi Studio connects straight from this Mac to MySQL, PostgreSQL or SQL Server. Nothing goes through a server of ours.'
          : 'Connections can be added in the Thabi Studio app.'
      "
    >
      <BaseButton
        variant="primary"
        :icon="Plus"
        :disabled="!nativeAvailable"
        @click="runCommand('connections.new')"
      >
        New Connection…
      </BaseButton>
    </EmptyState>
  </section>
</template>

<script setup>
import { Database, Plus } from "lucide-vue-next";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { runCommand } from "../../composables/useCommands.js";
import BaseButton from "../common/BaseButton.vue";
import EmptyState from "../common/EmptyState.vue";
import ConnectionProfilesList from "./ConnectionProfilesList.vue";

const { profiles, loaded, nativeAvailable } = useConnectionProfiles();
</script>

<template>
  <section class="mx-auto flex h-full max-w-4xl flex-col gap-5 p-8">
    <header class="flex items-start justify-between gap-4">
      <div class="flex flex-col gap-1">
        <h1 class="text-xl font-semibold text-fg">Connection Profiles</h1>
        <p class="max-w-2xl text-fg-muted">
          Profiles are stored on this Mac. Saving a profile doesn't connect to
          the database; connection testing arrives with database drivers.
        </p>
      </div>
      <BaseButton
        variant="primary"
        :icon="Plus"
        :disabled="!nativeAvailable"
        @click="runCommand('connections.new')"
      >
        New Connection…
      </BaseButton>
    </header>

    <ConnectionProfilesList v-if="profiles.length" />
    <div
      v-else-if="!loaded"
      class="py-8 text-center text-sm text-fg-subtle"
      role="status"
    >
      Loading saved connections…
    </div>
    <EmptyState
      v-else
      :icon="Database"
      title="No saved connections"
      :description="
        nativeAvailable
          ? 'Add a profile for MySQL, PostgreSQL or SQL Server. Nothing connects until you choose a real database driver.'
          : 'Connection profiles can be saved from the Thabi Studio desktop app.'
      "
      size="sm"
    />
  </section>
</template>

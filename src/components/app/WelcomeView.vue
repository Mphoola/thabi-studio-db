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
  <section
    v-if="profiles.length"
    class="mx-auto flex h-full max-w-4xl flex-col gap-5 p-8"
  >
    <header class="flex items-start justify-between gap-4">
      <div class="flex flex-col gap-1">
        <h1 class="text-xl font-semibold text-fg">Connections</h1>
        <p class="text-fg-muted">
          Saved on this Mac. Passwords stay in your Keychain.
        </p>
      </div>
      <BaseButton
        variant="primary"
        :icon="Plus"
        @click="runCommand('connections.new')"
      >
        New Connection…
      </BaseButton>
    </header>
    <ConnectionProfilesList />
  </section>
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

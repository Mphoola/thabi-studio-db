<script setup>
import { Database, Plus } from "lucide-vue-next";
import BaseButton from "../common/BaseButton.vue";
import EmptyState from "../common/EmptyState.vue";
import KbdShortcut from "../common/KbdShortcut.vue";
</script>

<template>
  <section class="flex h-full flex-col items-center justify-center">
    <EmptyState
      :icon="Database"
      title="Connect to your first database"
      description="Thabi Studio connects straight from this Mac to MySQL, PostgreSQL or SQL Server. Nothing goes through a server of ours."
    >
      <BaseButton variant="primary" :icon="Plus" disabled>
        New Connection…
      </BaseButton>
    </EmptyState>
    <p class="-mt-4 text-sm text-fg-subtle">
      Saving connections arrives in the next build. Press
      <KbdShortcut shortcut="Mod+P" /> to see what you can do now.
    </p>
  </section>
</template>

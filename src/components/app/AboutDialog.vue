<script setup>
import { words } from "../../lib/platform.js";
import { useAppInfo } from "../../composables/useAppInfo.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";

/** Version and the privacy note: what's stored, where, and what's sent. */
const open = defineModel("open", { type: Boolean, default: false });
const { info } = useAppInfo();
</script>

<template>
  <BaseDialog
    v-model:open="open"
    title="Thabi Studio"
    :description="info ? `Version ${info.version}` : undefined"
  >
    <div class="flex flex-col gap-3 text-fg-muted">
      <p>
        Thabi Studio connects directly from {{ words.computer }} to your
        database servers. It has no user accounts, no telemetry and no cloud
        backend. Nothing is sent anywhere except to the database and SSH servers
        you connect to.
      </p>
      <section class="flex flex-col gap-1.5">
        <h3 class="text-sm font-semibold text-fg">
          What stays on {{ words.computer }}
        </h3>
        <ul class="list-disc space-y-1 pl-5 text-sm">
          <li>
            Passwords and key passphrases, in {{ words.credentialStore }}.
          </li>
          <li>
            Connection settings, query history, saved queries, open tabs and
            preferences, in a local database.
          </li>
          <li>
            Query history contains the SQL you ran. You can turn it off or clear
            it in Settings.
          </li>
          <li>
            Query results are kept in memory only while their tab is open.
          </li>
        </ul>
        <p v-if="info?.dataDir" class="text-sm">
          Local data folder:
          <span class="selectable font-mono text-xs break-all text-fg">{{
            info.dataDir
          }}</span>
        </p>
      </section>
    </div>
    <template #footer>
      <BaseButton variant="primary" autofocus @click="open = false"
        >Done</BaseButton
      >
    </template>
  </BaseDialog>
</template>

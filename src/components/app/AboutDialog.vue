<script setup>
import { words } from "../../lib/platform.js";
import { useAppInfo } from "../../composables/useAppInfo.js";
import logoUrl from "../../assets/logo.svg";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";

/** Who the app is for and named after, its version, and the privacy note. */
const open = defineModel("open", { type: Boolean, default: false });
const { info } = useAppInfo();
const engines = ["MySQL", "PostgreSQL", "SQL Server"];
</script>

<template>
  <BaseDialog v-model:open="open" size="md" title="About Thabi Studio">
    <div class="flex flex-col gap-4 text-fg-muted">
      <header class="flex items-center gap-4">
        <img :src="logoUrl" alt="" class="size-20 shrink-0" />
        <div class="flex flex-col gap-1">
          <p class="text-xl font-semibold text-fg">Thabi Studio</p>
          <p class="text-sm">Version {{ info?.version ?? "1.0.0" }}</p>
          <p class="flex flex-wrap gap-1.5">
            <span
              v-for="engine in engines"
              :key="engine"
              class="rounded-sm border border-line px-1.5 text-xs text-fg-muted"
              >{{ engine }}</span
            >
          </p>
        </div>
      </header>

      <p class="text-base text-fg">
        A calm, local-first workspace for exploring, querying and editing your
        databases.
      </p>

      <section
        class="flex flex-col gap-1.5 rounded-lg bg-accent-subtle p-3 text-sm"
      >
        <h3 class="font-semibold text-accent-text">Named for Tabitha</h3>
        <p class="text-fg">
          Thabi is short for Tabitha Mphoola, the daughter of the developer who
          made it. Her name means “gazelle”: quick, light and sure-footed, which
          is what this app tries to be. The spark in the icon is hers.
        </p>
      </section>

      <section class="flex flex-col gap-1.5">
        <h3 class="text-sm font-semibold text-fg">Your data stays with you</h3>
        <p class="text-sm">
          Thabi Studio connects straight from {{ words.computer }} to your
          database and SSH servers. There are no accounts, no telemetry and no
          cloud backend.
        </p>
        <ul class="list-disc space-y-1 pl-5 text-sm">
          <li>
            Passwords and key passphrases are kept in
            {{ words.credentialStore }}.
          </li>
          <li>
            Connections, query history, saved queries, open tabs and preferences
            are kept in a local database. You can turn history off or clear it
            in Settings.
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

      <p class="text-xs text-fg-subtle">Made by Promise Mphoola.</p>
    </div>
    <template #footer>
      <BaseButton variant="primary" @click="open = false">Done</BaseButton>
    </template>
  </BaseDialog>
</template>

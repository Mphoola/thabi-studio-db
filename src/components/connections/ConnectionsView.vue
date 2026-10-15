<script setup>
import { words } from "../../lib/platform.js";
import { computed, ref } from "vue";
import { Plus, Search } from "lucide-vue-next";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useConnectionActions } from "../../composables/useConnectionActions.js";
import { runCommand } from "../../composables/useCommands.js";
import { ENVIRONMENTS } from "../../lib/environments.js";
import { relativeTime } from "../../lib/relativeTime.js";
import BaseButton from "../common/BaseButton.vue";
import KbdShortcut from "../common/KbdShortcut.vue";
import TextInput from "../common/TextInput.vue";
import ConnectionRow from "./ConnectionRow.vue";
import EngineBadge from "./EngineBadge.vue";

/** Saved connections: recent ones first, then grouped by environment. */
const store = useConnectionProfiles();
const actions = useConnectionActions();
const query = ref("");

const matches = (profile) => {
  const q = query.value.trim().toLowerCase();
  if (!q) return true;
  return [
    profile.name,
    profile.host,
    profile.databaseName,
    profile.username,
    profile.environment,
  ]
    .filter(Boolean)
    .some((text) => text.toLowerCase().includes(q));
};

// Production first: it's the one to notice.
const ORDER = ["production", "staging", "testing", "development", "local"];
const groups = computed(() =>
  ORDER.map((value) => ({
    ...ENVIRONMENTS.find((e) => e.value === value),
    profiles: store.profiles.value.filter(
      (p) => p.environment === value && matches(p),
    ),
  })).filter((group) => group.profiles.length),
);

const recent = computed(() =>
  [...store.profiles.value]
    .filter((p) => p.lastConnectedAt)
    .sort((a, b) => (a.lastConnectedAt < b.lastConnectedAt ? 1 : -1))
    .slice(0, 4),
);
const edge = {
  production: "border-l-tag-red",
  staging: "border-l-tag-orange",
  testing: "border-l-tag-teal",
  development: "border-l-tag-blue",
  local: "border-l-tag-gray",
};
</script>

<template>
  <section class="mx-auto flex max-w-4xl flex-col gap-5 p-8">
    <header class="flex flex-wrap items-center gap-3">
      <div class="mr-auto flex flex-col gap-0.5">
        <h1 class="text-xl font-semibold text-fg">Connections</h1>
        <p class="text-sm text-fg-muted">
          Saved on {{ words.computer }}. Double-click or press Return to
          connect. <KbdShortcut shortcut="Mod+Shift+K" /> switches from
          anywhere.
        </p>
      </div>
      <TextInput
        v-model="query"
        class="w-56"
        :icon="Search"
        placeholder="Search connections"
        aria-label="Search connections"
      />
      <BaseButton
        variant="primary"
        :icon="Plus"
        @click="runCommand('connections.new')"
      >
        New Connection…
      </BaseButton>
    </header>

    <section
      v-if="recent.length && !query"
      aria-label="Recently used"
      class="flex flex-col gap-2"
    >
      <h2 class="text-xs font-semibold tracking-wide text-fg-subtle uppercase">
        Recent
      </h2>
      <div class="flex flex-wrap gap-2">
        <button
          v-for="profile in recent"
          :key="profile.id"
          type="button"
          class="flex items-center gap-2 rounded-md border border-line bg-surface px-3 py-1.5 text-left hover:border-line-strong hover:bg-surface-hover"
          @click="actions.connect(profile)"
        >
          <EngineBadge :driver="profile.driver" compact />
          <span class="font-medium">{{ profile.name }}</span>
          <span class="text-xs text-fg-subtle">{{
            relativeTime(profile.lastConnectedAt)
          }}</span>
        </button>
      </div>
    </section>

    <section
      v-for="group in groups"
      :key="group.value"
      class="flex flex-col gap-2"
      :aria-label="group.label"
    >
      <h2
        class="flex items-center gap-2 text-xs font-semibold tracking-wide text-fg-subtle uppercase"
      >
        <span
          class="size-2 rounded-full"
          :class="group.dotClass"
          aria-hidden="true"
        />
        {{ group.label }}
        <span class="font-normal">{{ group.profiles.length }}</span>
      </h2>
      <ul
        class="flex flex-col divide-y divide-line overflow-hidden rounded-lg border border-l-4 border-line"
        :class="edge[group.value]"
      >
        <ConnectionRow
          v-for="profile in group.profiles"
          :key="profile.id"
          :profile="profile"
        />
      </ul>
    </section>
    <p v-if="!groups.length" class="py-8 text-center text-fg-subtle">
      No connections match “{{ query }}”.
    </p>
  </section>
</template>

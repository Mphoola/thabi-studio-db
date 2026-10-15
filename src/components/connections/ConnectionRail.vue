<script setup>
import { computed, ref } from "vue";
import { Database, Pencil, Plus, Unplug } from "lucide-vue-next";
import { runCommand } from "../../composables/useCommands.js";
import { useConnectionActions } from "../../composables/useConnectionActions.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { environmentFor } from "../../lib/environments.js";
import { formatShortcut } from "../../lib/shortcuts.js";
import DropdownMenu from "../common/DropdownMenu.vue";

/** Open connections down the left edge. Click one to make it active. */
const store = useConnectionProfiles();
const actions = useConnectionActions();
const menu = ref(null);
const target = ref(null);

const engineColour = {
  postgres: "text-engine-postgres",
  mysql: "text-engine-mysql",
  sqlserver: "text-engine-sqlserver",
};
const open = computed(() =>
  store.profiles.value.filter((p) => {
    const state = store.statusFor(p.id)?.state;
    return state === "connected" || state === "connecting";
  }),
);
const activeId = computed(() => store.selectedProfile.value?.id);

const items = [
  { id: "edit", label: "Edit Connection…", icon: Pencil },
  { separator: true },
  { id: "disconnect", label: "Disconnect", icon: Unplug },
];

function openMenu(event, profile) {
  target.value = profile;
  menu.value?.openAt(event.clientX, event.clientY);
}

function run(id) {
  if (id === "edit") store.openEdit(target.value.id);
  if (id === "disconnect") actions.disconnect(target.value);
}
</script>

<template>
  <nav
    v-if="open.length"
    class="flex w-16 shrink-0 flex-col items-center gap-1 overflow-y-auto border-r border-line bg-sidebar py-2"
    aria-label="Open connections"
  >
    <button
      v-for="profile in open"
      :key="profile.id"
      v-tooltip="
        `${profile.name} · ${environmentFor(profile.environment).label}`
      "
      type="button"
      class="relative flex w-14 flex-col items-center gap-1 rounded-md px-1 py-1.5 hover:bg-surface-hover focus-visible:bg-surface-hover"
      :class="profile.id === activeId && 'bg-surface-active'"
      :aria-current="profile.id === activeId ? 'true' : undefined"
      @click="actions.connect(profile)"
      @contextmenu.prevent="openMenu($event, profile)"
    >
      <span
        v-if="profile.id === activeId"
        class="absolute top-2 bottom-2 -left-1 w-0.5 rounded-full bg-accent"
        aria-hidden="true"
      />
      <span class="relative">
        <Database
          :size="20"
          :stroke-width="1.5"
          :class="engineColour[profile.driver] ?? 'text-fg-subtle'"
          aria-hidden="true"
        />
        <span
          class="absolute -right-0.5 -bottom-0.5 size-2 rounded-full ring-2 ring-sidebar"
          :class="
            store.statusFor(profile.id)?.state === 'connecting'
              ? 'animate-pulse bg-fg-subtle'
              : environmentFor(profile.environment).dotClass
          "
          aria-hidden="true"
        />
      </span>
      <span class="w-full truncate text-center text-xs text-fg-muted">
        {{ profile.name }}
      </span>
    </button>
    <button
      v-tooltip="`Switch Connection… ${formatShortcut('Mod+Shift+K')}`"
      type="button"
      class="mt-1 flex size-8 items-center justify-center rounded-md text-fg-subtle hover:bg-surface-hover hover:text-fg"
      aria-label="Switch Connection…"
      @click="runCommand('connections.switch')"
    >
      <Plus :size="16" :stroke-width="1.75" aria-hidden="true" />
    </button>
    <DropdownMenu
      ref="menu"
      :items="items"
      label="Connection actions"
      @select="run"
    />
  </nav>
</template>

<script setup>
import { defineAsyncComponent, onMounted } from "vue";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useAppCommands } from "../../composables/useAppCommands.js";
import { useDialogs } from "../../composables/useDialogs.js";
import { useLayout } from "../../composables/useLayout.js";
import AboutDialog from "./AboutDialog.vue";
import AppSidebar from "./AppSidebar.vue";
import CommandPalette from "./CommandPalette.vue";
import SettingsDialog from "./SettingsDialog.vue";
import ShortcutsDialog from "./ShortcutsDialog.vue";
import StatusBar from "./StatusBar.vue";
import TitleBar from "./TitleBar.vue";
import WelcomeView from "./WelcomeView.vue";
import ConnectionProfileDialog from "./ConnectionProfileDialog.vue";
import { useToasts } from "../../composables/useToasts.js";
import { toUserMessage } from "../../lib/errors.js";

// The component gallery is only bundled in development builds.
const ComponentGallery = import.meta.env.DEV
  ? defineAsyncComponent(() => import("../dev/ComponentGallery.vue"))
  : null;

useAppCommands();
const { sidebarOpen, mainView } = useLayout();
const dialogs = useDialogs();
const connections = useConnectionProfiles();
const toasts = useToasts();

onMounted(() => {
  connections.load().catch((error) => {
    toasts.error("Saved connections couldn't be loaded", toUserMessage(error));
  });
});

function dialogModel(name) {
  return {
    open: dialogs.open.value === name,
    "onUpdate:open": (value) => (value ? dialogs.show(name) : dialogs.close()),
  };
}
</script>

<template>
  <div class="flex h-screen flex-col bg-canvas text-fg">
    <TitleBar />
    <div class="flex min-h-0 flex-1">
      <AppSidebar v-if="sidebarOpen" />
      <main class="min-w-0 flex-1 overflow-auto" aria-label="Workspace">
        <component
          :is="ComponentGallery"
          v-if="ComponentGallery && mainView === 'gallery'"
        />
        <WelcomeView v-else />
      </main>
    </div>
    <StatusBar />

    <AboutDialog v-bind="dialogModel('about')" />
    <ConnectionProfileDialog v-bind="dialogModel('connection')" />
    <SettingsDialog v-bind="dialogModel('settings')" />
    <ShortcutsDialog v-bind="dialogModel('shortcuts')" />
    <CommandPalette v-bind="dialogModel('palette')" />
  </div>
</template>

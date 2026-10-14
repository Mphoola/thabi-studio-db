<script setup>
import { defineAsyncComponent } from "vue";
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

// The component gallery is only bundled in development builds.
const ComponentGallery = import.meta.env.DEV
  ? defineAsyncComponent(() => import("../dev/ComponentGallery.vue"))
  : null;

useAppCommands();
const { sidebarOpen, mainView } = useLayout();
const dialogs = useDialogs();

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
    <SettingsDialog v-bind="dialogModel('settings')" />
    <ShortcutsDialog v-bind="dialogModel('shortcuts')" />
    <CommandPalette v-bind="dialogModel('palette')" />
  </div>
</template>

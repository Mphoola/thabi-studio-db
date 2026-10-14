<script setup>
import { defineAsyncComponent, onBeforeUnmount, onMounted } from "vue";
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
import { onConnectionClosed } from "../../lib/tauri/events.js";
import EnvironmentStrip from "./EnvironmentStrip.vue";

// The component gallery is only bundled in development builds.
const ComponentGallery = import.meta.env.DEV
  ? defineAsyncComponent(() => import("../dev/ComponentGallery.vue"))
  : null;

useAppCommands();
const { sidebarOpen, mainView } = useLayout();
const dialogs = useDialogs();
const connections = useConnectionProfiles();
const toasts = useToasts();

let stopListening = () => {};

onMounted(async () => {
  connections.load().catch((error) => {
    toasts.error("Saved connections couldn’t be loaded", toUserMessage(error));
  });
  stopListening = await onConnectionClosed(({ profileId, reason }) => {
    const name =
      connections.profiles.value.find((p) => p.id === profileId)?.name ?? "";
    connections.forgetStatus(profileId);
    if (reason === "idle")
      toasts.info(`Disconnected from “${name}”`, "It was unused for an hour.");
    else
      toasts.error(
        `Lost the connection to “${name}”`,
        "The server stopped responding.",
      );
  });
});
onBeforeUnmount(() => stopListening());

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
    <EnvironmentStrip />
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

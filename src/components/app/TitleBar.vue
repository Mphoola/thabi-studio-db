<script setup>
import { computed } from "vue";
import {
  Command,
  Database,
  PanelLeft,
  Search,
  SquareTerminal,
} from "lucide-vue-next";
import { runCommand } from "../../composables/useCommands.js";
import { useLayout } from "../../composables/useLayout.js";
import { isTauri } from "../../lib/tauri/ipc.js";
import { isMac } from "../../lib/platform.js";
import IconButton from "../common/IconButton.vue";
import KbdShortcut from "../common/KbdShortcut.vue";
import ConnectionPill from "./ConnectionPill.vue";

const { sidebarOpen, mainView } = useLayout();
const showingQueries = computed(() => mainView.value === "workspace");
// On macOS the window has an overlay title bar; leave room for the traffic
// lights. Windows and Linux keep their native title bar.
const trafficLightInset = isTauri() && isMac;
</script>

<template>
  <header
    data-tauri-drag-region
    class="grid h-[38px] shrink-0 grid-cols-[1fr_auto_1fr] items-center gap-3 border-b border-line bg-toolbar px-2"
  >
    <div
      data-tauri-drag-region
      class="flex items-center"
      :class="trafficLightInset && 'pl-[72px]'"
    >
      <IconButton
        :icon="PanelLeft"
        label="Toggle Sidebar"
        shortcut="Mod+1"
        :pressed="sidebarOpen"
        @click="runCommand('view.toggleSidebar')"
      />
      <IconButton
        :icon="Database"
        label="Connections"
        :pressed="!showingQueries"
        @click="runCommand('view.connections')"
      />
      <IconButton
        :icon="SquareTerminal"
        label="Query Tabs"
        shortcut="Mod+T"
        :pressed="showingQueries"
        @click="runCommand(showingQueries ? 'tabs.newQuery' : 'view.workspace')"
      />
    </div>

    <ConnectionPill />

    <div data-tauri-drag-region class="flex items-center justify-end gap-1">
      <button
        type="button"
        class="flex h-6 w-52 items-center gap-2 rounded-md border border-line bg-surface px-2 text-sm text-fg-subtle transition-colors hover:border-line-strong hover:text-fg-muted"
        aria-label="Open Anything"
        @click="runCommand('app.commandPalette')"
      >
        <Search :size="13" :stroke-width="1.75" aria-hidden="true" />
        <span class="flex-1 text-left">Open anything…</span>
        <KbdShortcut shortcut="Mod+P" />
      </button>
      <IconButton
        :icon="Command"
        label="Keyboard Shortcuts"
        shortcut="Mod+/"
        size="sm"
        @click="runCommand('app.shortcuts')"
      />
    </div>
  </header>
</template>

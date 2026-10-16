<script setup>
import { SquareTerminal } from "lucide-vue-next";
import { useTabs } from "../../composables/useTabs.js";
import { runCommand } from "../../composables/useCommands.js";
import BaseButton from "../common/BaseButton.vue";
import EmptyState from "../common/EmptyState.vue";
import KbdShortcut from "../common/KbdShortcut.vue";
import ObjectView from "../explorer/ObjectView.vue";
import QueryTabView from "./QueryTabView.vue";
import TabBar from "./TabBar.vue";

const { tabs, active, activeId, splitPanes } = useTabs();
const visible = (tab) =>
  active.value?.id === tab.id || Boolean(splitPanes.value?.includes(tab.id));
</script>

<template>
  <div class="flex h-full min-h-0 flex-col">
    <TabBar v-if="tabs.length" />
    <div
      v-if="tabs.length"
      class="flex min-h-0 flex-1"
      :class="splitPanes ? 'divide-x divide-line' : 'flex-col'"
    >
      <template v-for="tab in tabs" :key="tab.id">
        <!-- Query tabs stay mounted so the editor keeps its undo history. -->
        <QueryTabView
          v-if="tab.type === 'query'"
          v-show="visible(tab)"
          :tab="tab"
          class="min-h-0 min-w-0 flex-1"
          :class="splitPanes && activeId !== tab.id && 'opacity-90'"
          :style="splitPanes ? { order: splitPanes.indexOf(tab.id) } : null"
          @focusin="activeId = tab.id"
        />
        <ObjectView
          v-else-if="active?.id === tab.id"
          :tab="tab"
          class="min-h-0 flex-1"
        />
      </template>
    </div>
    <section
      v-if="!tabs.length"
      class="flex h-full flex-col items-center justify-center"
    >
      <EmptyState :icon="SquareTerminal" title="No open queries">
        <BaseButton variant="primary" @click="runCommand('tabs.newQuery')">
          New Query Tab
        </BaseButton>
      </EmptyState>
      <p class="-mt-4 text-sm text-fg-subtle">
        Press <KbdShortcut shortcut="Mod+T" /> to start a query, or open a table
        from the explorer.
      </p>
    </section>
  </div>
</template>

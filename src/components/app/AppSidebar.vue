<script setup>
import { Bookmark, History, ListTree } from "lucide-vue-next";
import { useLayout } from "../../composables/useLayout.js";
import SegmentedControl from "../common/SegmentedControl.vue";
import ExplorerPanel from "../explorer/ExplorerPanel.vue";
import HistoryPanel from "../library/HistoryPanel.vue";
import SavedPanel from "../library/SavedPanel.vue";
import SidebarResizer from "./SidebarResizer.vue";

const { sidebarWidth, sidebarView } = useLayout();

const views = [
  { value: "explorer", label: "Explorer", icon: ListTree },
  { value: "saved", label: "Saved", icon: Bookmark },
  { value: "history", label: "History", icon: History },
];
</script>

<template>
  <aside
    class="relative flex shrink-0 flex-col border-r border-line bg-sidebar"
    :style="{ width: `${sidebarWidth}px` }"
    aria-label="Sidebar"
  >
    <div class="p-2">
      <SegmentedControl
        v-model="sidebarView"
        :options="views"
        label="Sidebar view"
        mode="tabs"
        panel-id-prefix="sidebar-panel"
        stretch
      />
    </div>
    <div
      :id="`sidebar-panel-${sidebarView}`"
      role="tabpanel"
      class="min-h-0 flex-1 overflow-auto"
    >
      <ExplorerPanel v-if="sidebarView === 'explorer'" />
      <SavedPanel v-else-if="sidebarView === 'saved'" />
      <HistoryPanel v-else />
    </div>
    <SidebarResizer />
  </aside>
</template>

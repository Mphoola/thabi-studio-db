<script setup>
import { Bookmark, History, ListTree } from "lucide-vue-next";
import { useLayout } from "../../composables/useLayout.js";
import EmptyState from "../common/EmptyState.vue";
import SegmentedControl from "../common/SegmentedControl.vue";
import ExplorerPanel from "../explorer/ExplorerPanel.vue";
import SidebarResizer from "./SidebarResizer.vue";

const { sidebarWidth, sidebarView } = useLayout();

const views = [
  { value: "explorer", label: "Explorer", icon: ListTree },
  { value: "saved", label: "Saved", icon: Bookmark },
  { value: "history", label: "History", icon: History },
];

const empty = {
  saved: {
    icon: Bookmark,
    title: "No saved queries",
    description: "Queries you save show up here for any connection.",
  },
  history: {
    icon: History,
    title: "No query history",
    description: "Queries you run are listed here. History stays on this Mac.",
  },
};
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
      <EmptyState
        v-else
        size="sm"
        :icon="empty[sidebarView].icon"
        :title="empty[sidebarView].title"
        :description="empty[sidebarView].description"
      />
    </div>
    <SidebarResizer />
  </aside>
</template>

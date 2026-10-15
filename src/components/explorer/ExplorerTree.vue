<script setup>
import { computed, nextTick, ref } from "vue";
import {
  ChevronRight,
  Database,
  Eye,
  Folder,
  Layers,
  Table2,
} from "lucide-vue-next";
import { useExplorer } from "../../composables/useExplorer.js";
import DropdownMenu from "../common/DropdownMenu.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";
import ExplorerNodeState from "./ExplorerNodeState.vue";
import { useExplorerActions } from "./useExplorerActions.js";

const explorer = useExplorer();
const actions = useExplorerActions();
const { rows, expanded, selectedKey } = explorer;
const tree = ref(null);
const menu = ref(null);
const menuNode = ref(null);

const icons = {
  database: Database,
  schema: Folder,
  group: Layers,
  table: Table2,
  view: Eye,
  materialized_view: Eye,
  foreign_table: Table2,
};
const iconFor = (node) =>
  node.type === "object" ? icons[node.kind] : icons[node.type];
const expandable = (node) => node.type !== "object";

const menuItems = computed(() => actions.menuFor(menuNode.value));

function activate(node) {
  selectedKey.value = node.key;
  if (node.type === "object") explorer.open(node, "data");
  else explorer.toggle(node);
}

function focusRow(index) {
  const items = tree.value?.querySelectorAll('[role="treeitem"]');
  const row = rows.value[index];
  if (!items?.[index] || !row) return;
  selectedKey.value = row.node.key;
  items[index].focus();
}

async function onKeydown(event, index) {
  const { node, level } = rows.value[index];
  const handlers = {
    ArrowDown: () => focusRow(Math.min(index + 1, rows.value.length - 1)),
    ArrowUp: () => focusRow(Math.max(index - 1, 0)),
    Home: () => focusRow(0),
    End: () => focusRow(rows.value.length - 1),
    ArrowRight: () => {
      if (!expandable(node)) return;
      if (!expanded.value.has(node.key)) explorer.expand(node, true);
      else focusRow(index + 1);
    },
    ArrowLeft: () => {
      if (expandable(node) && expanded.value.has(node.key))
        return explorer.expand(node, false);
      for (let i = index - 1; i >= 0; i -= 1)
        if (rows.value[i].level < level) return focusRow(i);
    },
    Enter: () => activate(node),
    " ": () => activate(node),
  };
  const handler = handlers[event.key];
  if (!handler) return;
  event.preventDefault();
  handler();
  await nextTick();
}

function openMenu(event, node) {
  event.preventDefault();
  selectedKey.value = node.key;
  menuNode.value = node;
  event.currentTarget.focus();
  const rect = event.currentTarget.getBoundingClientRect();
  const x = event.clientX || rect.left + 24;
  const y = event.clientY || rect.bottom;
  nextTick(() => menu.value?.openAt(x, y));
}
</script>

<template>
  <div class="overflow-auto pb-2">
    <ul ref="tree" role="tree" aria-label="Databases" class="px-1.5">
      <template v-for="({ node, level }, index) in rows" :key="node.key">
        <li
          role="treeitem"
          :aria-level="level"
          :aria-expanded="expandable(node) ? expanded.has(node.key) : undefined"
          :aria-selected="selectedKey === node.key"
          :tabindex="
            selectedKey === node.key || (!selectedKey && index === 0) ? 0 : -1
          "
          class="flex h-6 cursor-default items-center gap-1.5 rounded-md pr-2 text-sm whitespace-nowrap outline-none focus-visible:ring-2 focus-visible:ring-focus"
          :class="
            selectedKey === node.key
              ? 'bg-accent-subtle text-accent-text'
              : 'text-fg hover:bg-surface-hover'
          "
          :style="{ paddingLeft: `${(level - 1) * 14 + 4}px` }"
          @click="selectedKey = node.key"
          @dblclick="activate(node)"
          @keydown="onKeydown($event, index)"
          @contextmenu="openMenu($event, node)"
        >
          <button
            v-if="expandable(node)"
            type="button"
            tabindex="-1"
            class="flex size-4 items-center justify-center text-fg-subtle"
            :aria-label="expanded.has(node.key) ? 'Collapse' : 'Expand'"
            @click.stop="explorer.toggle(node)"
          >
            <ChevronRight
              :size="12"
              :stroke-width="2"
              class="transition-transform"
              :class="expanded.has(node.key) && 'rotate-90'"
            />
          </button>
          <span v-else class="w-4" />
          <component
            :is="iconFor(node)"
            :size="13"
            :stroke-width="1.75"
            class="shrink-0"
            :class="
              selectedKey === node.key ? 'text-accent-text' : 'text-fg-subtle'
            "
            aria-hidden="true"
          />
          <span class="truncate">{{ node.label }}</span>
          <span
            v-if="node.count !== undefined"
            class="ml-auto text-xs text-fg-subtle tabular-nums"
          >
            {{ node.count }}
          </span>
          <SpinnerIcon
            v-if="explorer.stateOf(node.key).status === 'loading'"
            :size="10"
            class="ml-auto"
          />
        </li>
        <ExplorerNodeState
          v-if="expanded.has(node.key) && !node.children"
          :node="node"
          :level="level + 1"
        />
      </template>
    </ul>
    <DropdownMenu
      ref="menu"
      :items="menuItems"
      label="Explorer actions"
      @select="(id) => actions.run(id, menuNode)"
    />
  </div>
</template>

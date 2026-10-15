<script setup>
import { computed, onMounted, ref } from "vue";
import {
  Bookmark,
  ClipboardCopy,
  Pencil,
  Search,
  SquareTerminal,
  Trash2,
} from "lucide-vue-next";
import { confirm } from "../../composables/useConfirm.js";
import { useLibrary } from "../../composables/useLibrary.js";
import { useQuerySave } from "../../composables/useQuerySave.js";
import { useTabs } from "../../composables/useTabs.js";
import { useToasts } from "../../composables/useToasts.js";
import { copyText } from "../../lib/clipboard.js";
import { fuzzyFilter } from "../../lib/fuzzy.js";
import { formatShortcut } from "../../lib/shortcuts.js";
import DropdownMenu from "../common/DropdownMenu.vue";
import EmptyState from "../common/EmptyState.vue";
import TextInput from "../common/TextInput.vue";

/** Saved queries in the sidebar. Click opens; right-click for more. */
const library = useLibrary();
const workspace = useTabs();
const saver = useQuerySave();
const toasts = useToasts();
const query = ref("");
const menu = ref(null);
const target = ref(null);

onMounted(() => library.load());

const results = computed(() =>
  fuzzyFilter(
    query.value,
    library.saved.value,
    (q) => `${q.name} ${q.notes} ${q.queryText}`,
  ),
);

const items = [
  { id: "open", label: "Open", icon: SquareTerminal },
  { id: "edit", label: "Rename…", icon: Pencil },
  { id: "copy", label: "Copy SQL", icon: ClipboardCopy },
  { separator: true },
  { id: "delete", label: "Delete…", icon: Trash2, danger: true },
];

function openMenu(event, saved) {
  event.preventDefault();
  target.value = saved;
  menu.value?.openAt(event.clientX, event.clientY);
}

async function run(id) {
  const saved = target.value;
  if (id === "open") workspace.openSaved(saved);
  if (id === "edit") saver.editSaved(saved);
  if (id === "copy")
    await copyText(saved.queryText).then(() => toasts.info("Copied SQL"));
  if (id === "delete") {
    const ok = await confirm({
      title: `Delete “${saved.name}”?`,
      message: "This removes the saved query. Open tabs keep their SQL.",
      confirmLabel: "Delete",
      destructive: true,
    });
    if (ok) await library.removeSaved(saved.id);
  }
}
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="px-2 pb-2">
      <TextInput
        v-model="query"
        :icon="Search"
        placeholder="Search saved queries"
        aria-label="Search saved queries"
      />
    </div>
    <EmptyState
      v-if="!library.saved.value.length"
      size="sm"
      :icon="Bookmark"
      title="No saved queries"
      :description="`Press ${formatShortcut('Mod+S')} in a query tab to save it here.`"
    />
    <ul
      v-else
      class="flex flex-col gap-0.5 overflow-auto px-1.5 pb-2"
      aria-label="Saved queries"
    >
      <li v-for="saved in results" :key="saved.id">
        <button
          type="button"
          class="flex w-full flex-col gap-0.5 rounded-md px-2 py-1.5 text-left hover:bg-surface-hover focus-visible:bg-surface-hover"
          @click="workspace.openSaved(saved)"
          @contextmenu="openMenu($event, saved)"
        >
          <span class="truncate text-sm font-medium text-fg">{{
            saved.name
          }}</span>
          <span class="truncate font-mono text-xs text-fg-subtle">
            {{ saved.notes || saved.queryText.split("\n")[0] }}
          </span>
        </button>
      </li>
      <li
        v-if="!results.length"
        class="px-2 py-4 text-center text-sm text-fg-subtle"
      >
        No saved queries match “{{ query }}”.
      </li>
    </ul>
    <DropdownMenu
      ref="menu"
      :items="items"
      label="Saved query actions"
      @select="run"
    />
  </div>
</template>

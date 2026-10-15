<script setup>
import { computed, onMounted, ref } from "vue";
import {
  Bookmark,
  ChevronRight,
  ClipboardCopy,
  Folder,
  FolderInput,
  FolderMinus,
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
import RenameFolderDialog from "./RenameFolderDialog.vue";
import SavedQueryItem from "./SavedQueryItem.vue";

/** Saved queries in the sidebar, grouped by folder. Right-click for more. */
const library = useLibrary();
const workspace = useTabs();
const saver = useQuerySave();
const toasts = useToasts();
const query = ref("");
const queryMenu = ref(null);
const folderMenu = ref(null);
const target = ref(null);
const folderTarget = ref("");
const renaming = ref(false);
// Collapsed folders, kept while the app runs.
const collapsed = ref(new Set());

onMounted(() => library.load());

const results = computed(() =>
  fuzzyFilter(
    query.value,
    library.saved.value,
    (q) => `${q.name} ${q.folder} ${q.notes} ${q.queryText}`,
  ),
);
/** Folders first (sorted), then queries without one. */
const groups = computed(() => {
  const byFolder = new Map(library.folders.value.map((f) => [f, []]));
  const loose = [];
  for (const saved of results.value)
    (saved.folder ? byFolder.get(saved.folder) : loose)?.push(saved);
  return [
    ...[...byFolder].filter(([, items]) => items.length),
    ...(loose.length ? [["", loose]] : []),
  ];
});
const isOpen = (folder) => !!query.value || !collapsed.value.has(folder);

function toggle(folder) {
  const next = new Set(collapsed.value);
  if (!next.delete(folder)) next.add(folder);
  collapsed.value = next;
}

const queryItems = [
  { id: "open", label: "Open", icon: SquareTerminal },
  { id: "edit", label: "Edit…", icon: Pencil },
  { id: "move", label: "Move to Folder…", icon: FolderInput },
  { id: "copy", label: "Copy SQL", icon: ClipboardCopy },
  { separator: true },
  { id: "delete", label: "Delete…", icon: Trash2, danger: true },
];
const folderItems = [
  { id: "rename", label: "Rename Folder…", icon: Pencil },
  { id: "ungroup", label: "Remove Folder…", icon: FolderMinus },
];

function openQueryMenu(event, saved) {
  target.value = saved;
  queryMenu.value?.openAt(event.clientX, event.clientY);
}

function openFolderMenu(event, folder) {
  folderTarget.value = folder;
  folderMenu.value?.openAt(event.clientX, event.clientY);
}

async function runQueryAction(id) {
  const saved = target.value;
  if (id === "open") workspace.openSaved(saved);
  if (id === "edit" || id === "move") saver.editSaved(saved);
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

async function runFolderAction(id) {
  const folder = folderTarget.value;
  if (id === "rename") renaming.value = true;
  if (id === "ungroup") {
    const ok = await confirm({
      title: `Remove the “${folder}” folder?`,
      message: "Its queries stay saved and move out of the folder.",
      confirmLabel: "Remove Folder",
    });
    if (ok) await library.renameFolder(folder, "");
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
    <div v-else class="flex flex-col overflow-auto px-1.5 pb-2">
      <template v-for="[folder, items] in groups" :key="folder || '(none)'">
        <button
          v-if="folder"
          type="button"
          class="flex h-7 w-full items-center gap-1.5 rounded-md px-2 text-left text-sm text-fg hover:bg-surface-hover focus-visible:bg-surface-hover"
          :aria-expanded="isOpen(folder)"
          @click="toggle(folder)"
          @contextmenu.prevent="openFolderMenu($event, folder)"
        >
          <ChevronRight
            :size="14"
            :stroke-width="1.75"
            class="shrink-0 text-fg-subtle transition-transform"
            :class="isOpen(folder) && 'rotate-90'"
            aria-hidden="true"
          />
          <Folder
            :size="14"
            :stroke-width="1.75"
            class="shrink-0 text-fg-subtle"
            aria-hidden="true"
          />
          <span class="min-w-0 flex-1 truncate font-medium">{{ folder }}</span>
          <span class="text-xs text-fg-subtle">{{ items.length }}</span>
        </button>
        <ul
          v-if="!folder || isOpen(folder)"
          class="flex flex-col gap-0.5"
          :aria-label="folder ? `Saved queries in ${folder}` : 'Saved queries'"
        >
          <li v-for="saved in items" :key="saved.id">
            <SavedQueryItem
              :saved="saved"
              :indented="!!folder"
              @open="workspace.openSaved"
              @menu="openQueryMenu"
            />
          </li>
        </ul>
      </template>
      <p
        v-if="!results.length"
        class="px-2 py-4 text-center text-sm text-fg-subtle"
      >
        No saved queries match “{{ query }}”.
      </p>
    </div>
    <DropdownMenu
      ref="queryMenu"
      :items="queryItems"
      label="Saved query actions"
      @select="runQueryAction"
    />
    <DropdownMenu
      ref="folderMenu"
      :items="folderItems"
      label="Folder actions"
      @select="runFolderAction"
    />
    <RenameFolderDialog v-model:open="renaming" :folder="folderTarget" />
  </div>
</template>

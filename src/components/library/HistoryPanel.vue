<script setup>
import { onMounted, ref, watch } from "vue";
import {
  CircleAlert,
  CircleCheck,
  CircleSlash,
  ClipboardCopy,
  Clock,
  History,
  Search,
  SquareTerminal,
  Trash2,
} from "lucide-vue-next";
import { confirm } from "../../composables/useConfirm.js";
import { useLibrary } from "../../composables/useLibrary.js";
import { useTabs } from "../../composables/useTabs.js";
import { useToasts } from "../../composables/useToasts.js";
import { copyText } from "../../lib/clipboard.js";
import { relativeTime } from "../../lib/relativeTime.js";
import BaseButton from "../common/BaseButton.vue";
import DropdownMenu from "../common/DropdownMenu.vue";
import EmptyState from "../common/EmptyState.vue";
import TextInput from "../common/TextInput.vue";

/** Queries you've run, newest first. Click to open in a new tab. */
const library = useLibrary();
const workspace = useTabs();
const toasts = useToasts();
const menu = ref(null);
const target = ref(null);
let timer = 0;

onMounted(() => library.load());
watch(library.historySearch, () => {
  clearTimeout(timer);
  timer = window.setTimeout(() => library.refreshHistory(), 200);
});

const icons = {
  completed: CircleCheck,
  failed: CircleAlert,
  cancelled: CircleSlash,
  timed_out: Clock,
};
const tones = {
  completed: "text-success-text",
  failed: "text-danger-text",
  cancelled: "text-fg-subtle",
  timed_out: "text-warning-text",
};

function open(entry) {
  workspace.newQuery({
    sql: entry.queryText,
    profileId: entry.profileId ?? undefined,
    database: entry.databaseName || null,
  });
}

function meta(entry) {
  const parts = [
    entry.connectionName,
    entry.databaseName,
    `${entry.durationMs.toLocaleString()} ms`,
  ];
  if (entry.rowCount !== null)
    parts.push(`${entry.rowCount.toLocaleString()} rows`);
  return parts.filter(Boolean).join(" · ");
}

const items = [
  { id: "open", label: "Open in New Tab", icon: SquareTerminal },
  { id: "copy", label: "Copy SQL", icon: ClipboardCopy },
  { separator: true },
  { id: "delete", label: "Delete", icon: Trash2, danger: true },
];

function openMenu(event, entry) {
  event.preventDefault();
  target.value = entry;
  menu.value?.openAt(event.clientX, event.clientY);
}

async function run(id) {
  const entry = target.value;
  if (id === "open") open(entry);
  if (id === "copy")
    await copyText(entry.queryText).then(() => toasts.info("Copied SQL"));
  if (id === "delete") await library.removeHistory(entry.id);
}

async function clearAll() {
  const ok = await confirm({
    title: "Clear query history?",
    message:
      "Every query in history is deleted from this computer. Saved queries aren't affected.",
    confirmLabel: "Clear History",
    destructive: true,
  });
  if (!ok) return;
  const count = await library.clearAll();
  toasts.info(
    `Cleared ${count.toLocaleString()} ${count === 1 ? "query" : "queries"}`,
  );
}
</script>

<template>
  <div class="flex h-full flex-col">
    <div class="px-2 pb-2">
      <TextInput
        v-model="library.historySearch.value"
        :icon="Search"
        placeholder="Search history"
        aria-label="Search history"
      />
    </div>
    <EmptyState
      v-if="!library.history.value.length && !library.historySearch.value"
      size="sm"
      :icon="History"
      title="No query history"
      description="Queries you run are listed here. History stays on this computer."
    />
    <ul
      v-else
      class="flex min-h-0 flex-1 flex-col gap-0.5 overflow-auto px-1.5 pb-2"
      aria-label="Query history"
    >
      <li v-for="entry in library.history.value" :key="entry.id">
        <button
          type="button"
          class="flex w-full gap-2 rounded-md px-2 py-1.5 text-left hover:bg-surface-hover focus-visible:bg-surface-hover"
          :title="entry.errorSummary ?? undefined"
          @click="open(entry)"
          @contextmenu="openMenu($event, entry)"
        >
          <component
            :is="icons[entry.status]"
            :size="12"
            :stroke-width="2"
            class="mt-0.5 shrink-0"
            :class="tones[entry.status]"
            :aria-label="entry.status"
          />
          <span class="flex min-w-0 flex-1 flex-col gap-0.5">
            <span class="line-clamp-2 font-mono text-xs break-words text-fg">{{
              entry.queryText
            }}</span>
            <span class="truncate text-xs text-fg-subtle">{{
              meta(entry)
            }}</span>
          </span>
          <span class="shrink-0 text-xs text-fg-subtle">{{
            relativeTime(entry.startedAt)
          }}</span>
        </button>
      </li>
      <li
        v-if="!library.history.value.length"
        class="px-2 py-4 text-center text-sm text-fg-subtle"
      >
        No queries match “{{ library.historySearch.value }}”.
      </li>
    </ul>
    <div v-if="library.history.value.length" class="border-t border-line p-2">
      <BaseButton
        size="sm"
        variant="ghost"
        :icon="Trash2"
        class="w-full"
        @click="clearAll"
      >
        Clear History…
      </BaseButton>
    </div>
    <DropdownMenu
      ref="menu"
      :items="items"
      label="History actions"
      @select="run"
    />
  </div>
</template>

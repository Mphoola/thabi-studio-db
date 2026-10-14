<script setup>
import { computed, nextTick, ref, useId, watch } from "vue";
import { Search } from "lucide-vue-next";
import { useCommands } from "../../composables/useCommands.js";
import { fuzzyFilter } from "../../lib/fuzzy.js";
import KbdShortcut from "../common/KbdShortcut.vue";

/** Open Anything (⌘P). Lists commands today; tables and saved queries join later. */
const open = defineModel("open", { type: Boolean, default: false });
const { commands, runCommand } = useCommands();

const dialog = ref(null);
const input = ref(null);
const query = ref("");
const active = ref(0);
const listId = useId();

const results = computed(() =>
  fuzzyFilter(
    query.value,
    commands.value.filter((c) => c.id !== "app.commandPalette"),
    (c) => `${c.group ?? ""} ${c.title}`,
  ),
);

watch(query, () => (active.value = 0));
watch(open, async (value) => {
  await nextTick();
  if (value && !dialog.value.open) {
    query.value = "";
    dialog.value.showModal();
    input.value.focus();
  } else if (!value && dialog.value.open) {
    dialog.value.close();
  }
});

function choose(command) {
  open.value = false;
  // Let the palette close before the command opens anything else.
  requestAnimationFrame(() => runCommand(command.id));
}

function onKeydown(event) {
  const count = results.value.length;
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    if (!count) return;
    active.value =
      (active.value + (event.key === "ArrowDown" ? 1 : -1) + count) % count;
    document
      .getElementById(`${listId}-${active.value}`)
      ?.scrollIntoView({ block: "nearest" });
  } else if (event.key === "Enter" && count) {
    event.preventDefault();
    choose(results.value[active.value]);
  }
}
</script>

<template>
  <dialog
    ref="dialog"
    aria-label="Open Anything"
    class="mx-auto mt-[12vh] w-[560px] max-w-[calc(100vw-48px)] rounded-lg bg-surface-raised p-0 text-fg shadow-dialog backdrop:bg-overlay"
    @close="open = false"
    @click="$event.target === dialog && (open = false)"
  >
    <div class="flex items-center gap-2 border-b border-line px-3">
      <Search
        :size="16"
        :stroke-width="1.75"
        class="text-fg-subtle"
        aria-hidden="true"
      />
      <input
        ref="input"
        v-model="query"
        role="combobox"
        aria-expanded="true"
        :aria-controls="listId"
        :aria-activedescendant="
          results.length ? `${listId}-${active}` : undefined
        "
        aria-autocomplete="list"
        placeholder="Search commands…"
        class="h-11 flex-1 bg-transparent text-lg text-fg outline-none placeholder:text-fg-subtle"
        spellcheck="false"
        autocomplete="off"
        @keydown="onKeydown"
      />
    </div>
    <ul
      :id="listId"
      role="listbox"
      aria-label="Commands"
      class="max-h-80 overflow-auto p-1"
    >
      <li
        v-for="(command, index) in results"
        :id="`${listId}-${index}`"
        :key="command.id"
        role="option"
        :aria-selected="index === active"
        class="flex h-8 cursor-default items-center gap-3 rounded-md px-2.5"
        :class="index === active ? 'bg-accent text-accent-fg' : 'text-fg'"
        @mousemove="active = index"
        @click="choose(command)"
      >
        <span class="flex-1 truncate">{{ command.title }}</span>
        <span
          class="text-sm"
          :class="index === active ? 'text-accent-fg' : 'text-fg-subtle'"
        >
          {{ command.group }}
        </span>
        <KbdShortcut v-if="command.shortcut" :shortcut="command.shortcut" />
      </li>
      <li v-if="!results.length" class="px-3 py-6 text-center text-fg-subtle">
        No commands match “{{ query }}”.
      </li>
    </ul>
  </dialog>
</template>

<script setup>
import { computed, nextTick, ref, useId, watch } from "vue";
import { Plug } from "lucide-vue-next";
import { useConnectionActions } from "../../composables/useConnectionActions.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { environmentFor } from "../../lib/environments.js";
import { fuzzyFilter } from "../../lib/fuzzy.js";
import EngineBadge from "./EngineBadge.vue";

/** Quick connection switcher (⇧⌘K): type, pick, connected. */
const open = defineModel("open", { type: Boolean, default: false });
const store = useConnectionProfiles();
const actions = useConnectionActions();
const dialog = ref(null);
const input = ref(null);
const query = ref("");
const active = ref(0);
const listId = useId();

const byRecent = computed(() =>
  [...store.profiles.value].sort((a, b) =>
    (b.lastConnectedAt ?? "").localeCompare(a.lastConnectedAt ?? ""),
  ),
);
const results = computed(() =>
  fuzzyFilter(
    query.value,
    byRecent.value,
    (p) => `${p.name} ${p.host} ${p.databaseName} ${p.environment}`,
  ),
);

watch(query, () => (active.value = 0));
watch(open, async (value) => {
  await nextTick();
  if (value && !dialog.value.open) {
    query.value = "";
    dialog.value.showModal();
    input.value.focus();
  } else if (!value && dialog.value.open) dialog.value.close();
});

function choose(profile) {
  open.value = false;
  actions.connect(profile);
}

function onKeydown(event) {
  const count = results.value.length;
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    if (count)
      active.value =
        (active.value + (event.key === "ArrowDown" ? 1 : -1) + count) % count;
  } else if (event.key === "Enter" && count) {
    event.preventDefault();
    choose(results.value[active.value]);
  }
}
</script>

<template>
  <dialog
    ref="dialog"
    aria-label="Switch Connection"
    class="mx-auto mt-[12vh] w-[520px] max-w-[calc(100vw-48px)] rounded-lg bg-surface-raised p-0 text-fg shadow-dialog backdrop:bg-overlay"
    @close="open = false"
    @click="$event.target === dialog && (open = false)"
  >
    <div class="flex items-center gap-2 border-b border-line px-3">
      <Plug
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
        placeholder="Switch to connection…"
        class="h-11 flex-1 bg-transparent text-lg text-fg outline-none placeholder:text-fg-subtle"
        spellcheck="false"
        autocomplete="off"
        @keydown="onKeydown"
      />
    </div>
    <ul
      :id="listId"
      role="listbox"
      aria-label="Connections"
      class="max-h-80 overflow-auto p-1"
    >
      <li
        v-for="(profile, index) in results"
        :id="`${listId}-${index}`"
        :key="profile.id"
        role="option"
        :aria-selected="index === active"
        class="flex h-9 cursor-default items-center gap-3 rounded-md px-2.5"
        :class="index === active ? 'bg-accent-subtle' : ''"
        @mousemove="active = index"
        @click="choose(profile)"
      >
        <span
          class="size-2 shrink-0 rounded-full"
          :class="environmentFor(profile.environment).dotClass"
          aria-hidden="true"
        />
        <span class="truncate font-medium">{{ profile.name }}</span>
        <EngineBadge :driver="profile.driver" compact />
        <span class="ml-auto truncate font-mono text-xs text-fg-subtle">{{
          profile.host
        }}</span>
        <span
          v-if="store.isConnected(profile.id)"
          class="text-xs text-success-text"
          >Connected</span
        >
      </li>
      <li v-if="!results.length" class="px-3 py-6 text-center text-fg-subtle">
        No connections match “{{ query }}”.
      </li>
    </ul>
  </dialog>
</template>

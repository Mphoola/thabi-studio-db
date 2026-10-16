<script setup>
import { computed, ref } from "vue";
import { ChevronDown, ChevronRight, Plus, Search } from "lucide-vue-next";
import { words } from "../../lib/platform.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useConnectionActions } from "../../composables/useConnectionActions.js";
import { runCommand } from "../../composables/useCommands.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserMessage } from "../../lib/errors.js";
import {
  groupConnections,
  groupEdgeClass,
  isCollapsed,
  reorderWithin,
} from "../../lib/connectionGroups.js";
import { relativeTime } from "../../lib/relativeTime.js";
import { setAppSetting } from "../../lib/tauri/commands.js";
import BaseButton from "../common/BaseButton.vue";
import KbdShortcut from "../common/KbdShortcut.vue";
import TextInput from "../common/TextInput.vue";
import ConnectionRow from "./ConnectionRow.vue";
import EngineBadge from "./EngineBadge.vue";

/**
 * Saved connections: recent first, then the user's own groups and the
 * environment groups for anything ungrouped. Rows drag to reorder inside
 * their group, or move with Alt+Up / Alt+Down from the keyboard.
 */
const store = useConnectionProfiles();
const actions = useConnectionActions();
const toasts = useToasts();
const query = ref("");
/** User groups the user has folded away, by name. */
const collapsed = ref([]);
/** The profile being dragged, or the one being moved with the keyboard. */
const pending = ref(null);

const matches = (profile) => {
  const q = query.value.trim().toLowerCase();
  if (!q) return true;
  return [
    profile.name,
    profile.host,
    profile.databaseName,
    profile.username,
    profile.environment,
    profile.groupName,
  ]
    .filter(Boolean)
    .some((text) => text.toLowerCase().includes(q));
};

const groups = computed(() => groupConnections(store.profiles.value, matches));
const recent = computed(() =>
  [...store.profiles.value]
    .filter((p) => p.lastConnectedAt)
    .sort((a, b) => (a.lastConnectedAt < b.lastConnectedAt ? 1 : -1))
    .slice(0, 4),
);

function toggleGroup(group) {
  // Environment sections are a fixed reference; only user groups fold.
  if (!group.user) return;
  const folded = collapsed.value.includes(group.key);
  collapsed.value = folded
    ? collapsed.value.filter((name) => name !== group.key)
    : [...collapsed.value, group.key];
  setAppSetting("collapsed_groups", collapsed.value).catch(() => {});
}

/** Saves a new order after a drag, or after Alt+Up / Alt+Down. */
async function move(draggedId, targetId) {
  pending.value = null;
  if (!draggedId || !targetId || draggedId === targetId) return;
  try {
    await store.reorder(
      reorderWithin(
        store.profiles.value.map((profile) => profile.id),
        draggedId,
        targetId,
      ),
    );
  } catch (error) {
    toasts.error("Couldn't save the new order", toUserMessage(error));
  }
}

/** Alt+Up / Alt+Down move a focused row one place along. */
function moveBy(profile, direction) {
  pending.value = profile.id;
  const index = store.profiles.value.findIndex((p) => p.id === profile.id);
  const target = store.profiles.value[index + direction];
  if (target) move(profile.id, target.id);
}
</script>

<template>
  <section class="mx-auto flex max-w-4xl flex-col gap-5 p-8">
    <header class="flex flex-wrap items-center gap-3">
      <div class="mr-auto flex flex-col gap-0.5">
        <h1 class="text-xl font-semibold text-fg">Connections</h1>
        <p class="text-sm text-fg-muted">
          Saved on {{ words.computer }}. Double-click or press Return to
          connect. <KbdShortcut shortcut="Mod+Shift+K" /> switches from
          anywhere.
        </p>
      </div>
      <TextInput
        v-model="query"
        class="w-56"
        :icon="Search"
        placeholder="Search connections"
        aria-label="Search connections"
      />
      <BaseButton
        variant="primary"
        :icon="Plus"
        @click="runCommand('connections.new')"
      >
        New Connection…
      </BaseButton>
    </header>

    <section
      v-if="recent.length && !query"
      aria-label="Recently used"
      class="flex flex-col gap-2"
    >
      <h2 class="text-xs font-semibold tracking-wide text-fg-subtle uppercase">
        Recent
      </h2>
      <div class="flex flex-wrap gap-2">
        <button
          v-for="profile in recent"
          :key="profile.id"
          type="button"
          class="flex items-center gap-2 rounded-md border border-line bg-surface px-3 py-1.5 text-left hover:border-line-strong hover:bg-surface-hover"
          @click="actions.connect(profile)"
        >
          <EngineBadge :driver="profile.driver" compact />
          <span class="font-medium">{{ profile.name }}</span>
          <span class="text-xs text-fg-subtle">{{
            relativeTime(profile.lastConnectedAt)
          }}</span>
        </button>
      </div>
    </section>

    <section
      v-for="group in groups"
      :key="group.key"
      class="flex flex-col gap-2"
      :aria-label="group.label"
    >
      <h2 class="flex items-center gap-1.5">
        <component
          :is="group.user ? 'button' : 'span'"
          v-bind="
            group.user
              ? {
                  type: 'button',
                  'aria-expanded': !isCollapsed(collapsed, group),
                  onclick: () => toggleGroup(group),
                }
              : {}
          "
          class="flex items-center gap-1.5 text-xs font-semibold tracking-wide text-fg-subtle uppercase"
          :class="group.user && 'rounded hover:text-fg focus-visible:text-fg'"
        >
          <component
            :is="isCollapsed(collapsed, group) ? ChevronRight : ChevronDown"
            v-if="group.user"
            :size="14"
            :stroke-width="2"
            aria-hidden="true"
          />
          {{ group.label }}
        </component>
        <span class="text-xs font-normal text-fg-subtle">{{
          group.profiles.length
        }}</span>
      </h2>
      <ul
        v-if="!isCollapsed(collapsed, group)"
        class="flex flex-col divide-y divide-line overflow-hidden rounded-lg border border-l-4 border-line"
        :class="groupEdgeClass(group)"
      >
        <ConnectionRow
          v-for="profile in group.profiles"
          :key="profile.id"
          :profile="profile"
          :moving="pending === profile.id"
          draggable="true"
          @dragstart="pending = profile.id"
          @dragend="pending = null"
          @dragover.prevent
          @drop="move(pending, profile.id)"
          @move="moveBy(profile, $event)"
        />
      </ul>
    </section>
    <p v-if="!groups.length" class="py-8 text-center text-fg-subtle">
      No connections match “{{ query }}”.
    </p>
  </section>
</template>

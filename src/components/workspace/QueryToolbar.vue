<script setup>
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { ChevronDown, Columns2, Database, Play, Square } from "lucide-vue-next";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { vTooltip } from "../../composables/useTooltip.js";
import { environmentFor } from "../../lib/environments.js";
import { listDatabases } from "../../lib/tauri/commands.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";
import KbdShortcut from "../common/KbdShortcut.vue";
import { useTabs } from "../../composables/useTabs.js";

/** Run controls and the connection/database the tab will run against. */
const props = defineProps({ tab: { type: Object, required: true } });
defineEmits(["run", "run-all", "cancel"]);

const store = useConnectionProfiles();
const workspace = useTabs();
// Split Editor on and off from the toolbar too (⇧⌘D does the same).
const inSplit = computed(() =>
  Boolean(workspace.splitPanes.value?.includes(props.tab.id)),
);
const profile = computed(() =>
  store.profiles.value.find((p) => p.id === props.tab.profileId),
);
const connected = computed(
  () => profile.value && store.isConnected(profile.value.id),
);
const databases = ref([]);
const elapsed = ref(0);
let timer = 0;

watch(
  connected,
  async (isConnected) => {
    databases.value = [];
    if (!isConnected) return;
    // A connection saved with a database offers just that database.
    if (profile.value.databaseName) {
      databases.value = [profile.value.databaseName];
      return;
    }
    try {
      databases.value = await listDatabases(profile.value.id);
    } catch {
      databases.value = [];
    }
  },
  { immediate: true },
);

const databaseOptions = computed(() => {
  const names = new Set(databases.value);
  if (props.tab.database) names.add(props.tab.database);
  return [...names];
});

watch(
  () => props.tab.running,
  (running) => {
    clearInterval(timer);
    elapsed.value = 0;
    if (running)
      timer = window.setInterval(() => {
        elapsed.value = (Date.now() - props.tab.startedAt) / 1000;
      }, 100);
  },
);
onBeforeUnmount(() => clearInterval(timer));

const cancelHint = computed(() =>
  profile.value?.driver === "sqlserver"
    ? "Stops the query by resetting this tab’s session"
    : "Asks the server to stop the query",
);
</script>

<template>
  <div class="flex h-9 shrink-0 items-center gap-2 border-b border-line px-2.5">
    <BaseButton
      v-if="!tab.running"
      variant="primary"
      size="sm"
      :icon="Play"
      @click="$emit('run')"
    >
      Run
      <KbdShortcut shortcut="Mod+Enter" class="bg-transparent text-accent-fg" />
    </BaseButton>
    <BaseButton
      v-else
      v-tooltip="cancelHint"
      variant="danger"
      size="sm"
      :icon="Square"
      @click="$emit('cancel')"
    >
      Cancel
      <KbdShortcut
        shortcut="Mod+Period"
        class="bg-transparent text-danger-fg"
      />
    </BaseButton>
    <BaseButton
      variant="ghost"
      size="sm"
      :disabled="tab.running"
      @click="$emit('run-all')"
    >
      Run All
    </BaseButton>
    <span
      v-if="tab.running"
      class="text-sm text-fg-muted tabular-nums"
      role="status"
    >
      Running… {{ elapsed.toFixed(1) }} s
    </span>

    <span class="ml-auto h-4 w-px bg-line" aria-hidden="true" />
    <span
      v-if="profile"
      class="flex min-w-0 items-center gap-1.5 text-sm text-fg-muted"
    >
      <span
        class="size-2 shrink-0 rounded-full"
        :class="environmentFor(profile.environment).dotClass"
        aria-hidden="true"
      />
      <span class="truncate font-medium text-fg">{{ profile.name }}</span>
      <span v-if="!connected" class="text-fg-subtle">· Not connected</span>
    </span>
    <span v-else class="text-sm text-warning-text">No connection selected</span>
    <label
      v-tooltip="'Database this tab runs in'"
      class="relative flex items-center text-sm text-fg-muted"
    >
      <Database
        :size="13"
        :stroke-width="1.75"
        class="pointer-events-none absolute left-2 text-fg-subtle"
        aria-hidden="true"
      />
      <span class="sr-only">Database</span>
      <select
        v-model="tab.database"
        class="h-6 max-w-48 appearance-none rounded-md border border-line-strong bg-surface pr-6 pl-7 text-sm font-medium text-fg hover:border-fg-subtle disabled:opacity-50"
        :disabled="tab.running || !databaseOptions.length"
        aria-label="Database"
      >
        <option v-if="!databaseOptions.length" :value="tab.database">
          {{ tab.database ?? "Default database" }}
        </option>
        <option v-for="name in databaseOptions" :key="name" :value="name">
          {{ name }}
        </option>
      </select>
      <ChevronDown
        :size="12"
        class="pointer-events-none absolute right-1.5 text-fg-subtle"
        aria-hidden="true"
      />
    </label>
    <IconButton
      :icon="Columns2"
      size="sm"
      :label="inSplit ? 'Close Split' : 'Split Editor'"
      shortcut="Mod+Shift+D"
      :pressed="inSplit"
      @click="
        workspace.activeId.value = tab.id;
        workspace.toggleSplit();
      "
    />
  </div>
</template>

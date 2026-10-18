<script setup>
import { computed, onMounted, ref, watch } from "vue";
import { ListTree, RefreshCw, Table2, Unplug } from "lucide-vue-next";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useExplorer } from "../../composables/useExplorer.js";
import { useExplorerActions } from "./useExplorerActions.js";
import SqlExportDialog from "./SqlExportDialog.vue";
import { listObjects } from "../../lib/tauri/commands.js";
import { useHostKeyTrust } from "../../composables/useHostKeyTrust.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail } from "../../lib/errors.js";
import BaseButton from "../common/BaseButton.vue";
import EmptyState from "../common/EmptyState.vue";
import IconButton from "../common/IconButton.vue";
import InlineAlert from "../common/InlineAlert.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";
import ExplorerTree from "./ExplorerTree.vue";
import CreateTableDialog from "./CreateTableDialog.vue";

const explorer = useExplorer();
const exportActions = useExplorerActions();
const connections = useConnectionProfiles();
const creatingTable = ref(false);

/** Where a new table would go: the selected database or schema. */
const newTableTarget = computed(() => {
  const profileId = explorer.profile.value?.id;
  const profile = connections.profiles.value.find((p) => p.id === profileId);
  if (!profile) return null;
  const selected = explorer.rows.value
    .flatMap((row) => row.children ?? [])
    .find((child) => child.node.key === explorer.selectedKey.value)?.node;
  if (!selected) return null;
  // A table or view is not a home for a new one; its parent is.
  if (selected.kind === "table" || selected.kind === "view") return null;
  return {
    profileId,
    database: selected.database ?? profile.databaseName ?? "",
    schema: selected.schema ?? "",
    driver: profile.driver,
  };
});
const store = useConnectionProfiles();
const hostKeys = useHostKeyTrust();
const toasts = useToasts();
const { profile, status, connected, rootState } = explorer;

// Load databases whenever a connection becomes available.
/** Table names in whatever the export dialog is pointed at, for its list. */
const exportTables = ref([]);

async function loadExportTables() {
  const target = exportActions.target.value;
  if (!target) {
    exportTables.value = [];
    return;
  }
  // A single table does not need a list; asking for one would list the whole
  // schema to show one checkbox.
  if (target.tables.length) {
    exportTables.value = target.tables;
    return;
  }
  try {
    const objects = await listObjects(
      // The profile, not the connection: every object command is keyed by
      // profile id, and a connection id finds no session — which came back as
      // an empty list rather than as an error, so the backup panel said "0
      // tables" about a database full of them.
      store.selectedProfile.value?.id,
      target.database,
      target.schema || null,
    );
    exportTables.value = objects
      .filter((one) => one.kind === "table")
      .map((one) => one.name);
  } catch {
    // No list is not a reason to refuse the export; an empty schema has no
    // tables to choose between either.
    exportTables.value = [];
  }
}

// Re-read the list whenever the dialog opens for somewhere new.
watch(
  () => exportActions.open.value,
  (isOpen) => {
    if (isOpen) loadExportTables();
  },
);

watch(
  () => [connected.value, status.value?.connectionId],
  () => {
    explorer.syncConnection();
    if (connected.value && rootState.value.status === "idle") explorer.load();
  },
);
onMounted(() => {
  explorer.syncConnection();
  if (connected.value && rootState.value.status === "idle") explorer.load();
});

async function connect() {
  try {
    await hostKeys.run(() => store.connect(profile.value.id));
  } catch (error) {
    toasts.error(
      `Couldn’t connect to “${profile.value.name}”`,
      toUserDetail(error),
    );
  }
}
</script>

<template>
  <div class="flex h-full flex-col">
    <EmptyState
      v-if="!profile"
      size="sm"
      :icon="ListTree"
      title="Nothing to explore yet"
      description="Choose a connection to browse its databases, schemas and tables."
    />
    <EmptyState
      v-else-if="!connected"
      size="sm"
      :icon="Unplug"
      title="Not connected"
      :description="`Connect to “${profile.name}” to browse its databases.`"
    >
      <BaseButton
        size="sm"
        :loading="status?.state === 'connecting'"
        @click="connect"
      >
        Connect
      </BaseButton>
    </EmptyState>
    <template v-else>
      <div class="flex h-7 shrink-0 items-center justify-between px-3">
        <span class="truncate text-xs text-fg-subtle">
          {{ status.server?.version }}
        </span>
        <div class="flex items-center gap-1">
          <IconButton
            :icon="Table2"
            size="sm"
            label="New Table"
            :disabled="!newTableTarget"
            :title="
              newTableTarget
                ? `Create a table in ${newTableTarget.schema || newTableTarget.database}`
                : 'Select a database or schema first'
            "
            @click="creatingTable = true"
          />
          <IconButton
            :icon="RefreshCw"
            size="sm"
            label="Refresh Explorer"
            shortcut="Mod+R"
            @click="explorer.refresh()"
          />
        </div>
      </div>
      <CreateTableDialog
        v-if="newTableTarget"
        v-model:open="creatingTable"
        :profile-id="newTableTarget.profileId"
        :tab-id="`create:${newTableTarget.database}:${newTableTarget.schema}`"
        :database="newTableTarget.database"
        :schema="newTableTarget.schema"
        :engine="newTableTarget.driver"
        @created="explorer.refresh()"
      />
      <div
        v-if="rootState.status === 'loading' && !rootState.children"
        class="flex items-center gap-2 px-3 py-2 text-sm text-fg-subtle"
        role="status"
      >
        <SpinnerIcon :size="12" /> Loading databases…
      </div>
      <div v-else-if="rootState.status === 'error'" class="p-2">
        <InlineAlert
          tone="danger"
          :title="rootState.error?.message ?? 'Couldn’t load databases.'"
          :message="rootState.error?.detail ?? ''"
        >
          <BaseButton
            size="sm"
            @click="
              rootState.error?.kind === 'connection_lost'
                ? connect()
                : explorer.refresh()
            "
          >
            {{
              rootState.error?.kind === "connection_lost"
                ? "Reconnect"
                : "Try Again"
            }}
          </BaseButton>
        </InlineAlert>
      </div>
      <ExplorerTree v-else class="min-h-0 flex-1" />

      <SqlExportDialog
        v-if="exportActions.target.value"
        v-model:open="exportActions.open.value"
        :profile-id="exportActions.target.value.profileId"
        :database="exportActions.target.value.database"
        :schema="exportActions.target.value.schema ?? ''"
        :engine="exportActions.target.value.engine"
        :tables="exportActions.target.value.tables"
        :available="exportTables"
        @done="loadExportTables()"
      />
    </template>
  </div>
</template>

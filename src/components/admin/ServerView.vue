<script setup>
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { RefreshCw } from "lucide-vue-next";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { toUserDetail } from "../../lib/errors.js";
import {
  listProcesses,
  serverOverview,
  storageOverview,
} from "../../lib/tauri/commands.js";
import IconButton from "../common/IconButton.vue";
import InlineAlert from "../common/InlineAlert.vue";
import SegmentedControl from "../common/SegmentedControl.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";
import ActivityTable from "./ActivityTable.vue";
import StorageTable from "./StorageTable.vue";
import UsersPanel from "./UsersPanel.vue";

/**
 * A connection's server: its sessions (refreshed every few seconds while
 * shown), its vital signs, and where the space in a schema goes.
 */
const props = defineProps({ tab: { type: Object, required: true } });
const emit = defineEmits(["open-table"]);

const connections = useConnectionProfiles();
const profile = computed(
  () =>
    connections.profiles.value.find((p) => p.id === props.tab.profileId) ??
    null,
);
const view = computed({
  get: () => props.tab.view ?? "activity",
  set: (value) => (props.tab.view = value),
});
const views = computed(() => [
  { value: "activity", label: "Activity" },
  { value: "server", label: "Server" },
  ...(props.tab.database ? [{ value: "storage", label: "Storage" }] : []),
  { value: "users", label: "Users" },
]);
// Users are read through a session in a database; SQL Server's live there.
const usersDatabase = computed(
  () =>
    props.tab.database ||
    profile.value?.databaseName ||
    { postgres: "postgres", mysql: "mysql", sqlserver: "master" }[
      profile.value?.driver
    ] ||
    "",
);

const loading = ref(false);
const error = ref("");
const processes = ref([]);
const facts = ref([]);
const tables = ref([]);
const live = ref(true);
let timer = null;

async function load() {
  loading.value = true;
  error.value = "";
  const id = props.tab.profileId;
  if (view.value === "users") return void (loading.value = false);
  try {
    if (view.value === "activity") processes.value = await listProcesses(id);
    else if (view.value === "server") facts.value = await serverOverview(id);
    else
      tables.value = await storageOverview(
        id,
        props.tab.database,
        props.tab.schema ?? undefined,
      );
  } catch (failure) {
    error.value = toUserDetail(failure);
  } finally {
    loading.value = false;
  }
}

// Sessions come and go; the list follows them while it's being looked at.
function schedule() {
  clearInterval(timer);
  if (view.value === "activity" && live.value)
    timer = setInterval(() => !loading.value && load(), 5000);
}
watch(view, () => {
  load();
  schedule();
});
watch(live, schedule);
load();
schedule();
onBeforeUnmount(() => clearInterval(timer));
</script>

<template>
  <div class="flex min-h-0 flex-col">
    <header class="flex items-center gap-3 border-b border-line px-4 py-2">
      <h2 class="text-base font-semibold">{{ profile?.name ?? "Server" }}</h2>
      <SegmentedControl v-model="view" :options="views" label="Show" />
      <span v-if="view === 'storage'" class="text-sm text-fg-subtle">{{
        [tab.database, tab.schema].filter(Boolean).join(".")
      }}</span>
      <div class="ml-auto flex items-center gap-2">
        <label
          v-if="view === 'activity'"
          class="flex items-center gap-1.5 text-sm text-fg-muted"
        >
          <input v-model="live" type="checkbox" />
          Refresh every 5 s
        </label>
        <SpinnerIcon v-if="loading" :size="14" />
        <IconButton
          v-if="view !== 'users'"
          :icon="RefreshCw"
          label="Refresh"
          @click="load"
        />
      </div>
    </header>
    <InlineAlert
      v-if="error"
      class="m-4"
      tone="danger"
      title="Couldn't read the server"
      :message="error"
    />
    <ActivityTable
      v-else-if="view === 'activity'"
      :processes="processes"
      :profile-id="tab.profileId"
      :engine="profile?.driver"
      :read-only="profile?.safeMode === 'read-only'"
      :connection="profile?.name ?? ''"
      @changed="load"
    />
    <dl
      v-else-if="view === 'server'"
      class="grid max-w-3xl grid-cols-[12rem_1fr] gap-x-6 gap-y-2 p-4 text-sm"
    >
      <template v-for="fact in facts" :key="fact.label">
        <dt class="text-fg-muted">{{ fact.label }}</dt>
        <dd class="font-mono text-xs break-all">{{ fact.value || "—" }}</dd>
      </template>
    </dl>
    <UsersPanel
      v-else-if="view === 'users' && profile"
      :key="usersDatabase"
      :profile-id="tab.profileId"
      :database="usersDatabase"
      :engine="profile.driver"
      :read-only="profile.safeMode === 'read-only'"
    />
    <StorageTable
      v-else-if="view === 'storage'"
      :tables="tables"
      @open-table="emit('open-table', $event)"
    />
  </div>
</template>

<script setup>
/**
 * Choosing what a SQL export contains, and starting it.
 *
 * Three things are said here that a file picker cannot, each of which changes
 * what someone believes about the file they end up with: MySQL's structure is
 * the server's own CREATE TABLE while the other two are rebuilt from the
 * catalog and are blind to triggers; every file starts with DROP TABLE IF
 * EXISTS, so running it over a live database replaces tables; and a row limit
 * is a sample, not a backup, so it is off by default.
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { FileCode2 } from "lucide-vue-next";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import BackupForm from "./BackupForm.vue";
import ExportProgress from "./ExportProgress.vue";
import InlineAlert from "../common/InlineAlert.vue";
import { useToasts } from "../../composables/useToasts.js";
import { cancelSqlExport, exportSql } from "../../lib/tauri/commands.js";
import {
  defaultBackupPath,
  pickSqlDestination,
} from "../../lib/tauri/files.js";
import { toUserDetail } from "../../lib/errors.js";

const props = defineProps({
  /** The connection's profile id, which is what commands are keyed by. */
  profileId: { type: String, required: true },
  database: { type: String, required: true },
  schema: { type: String, default: "" },
  engine: { type: String, default: "" },
  /** Pre-selected tables. Empty means the whole schema. */
  tables: { type: Array, default: () => [] },
  /** Names in this schema, for the list. */
  available: { type: Array, default: () => [] },
});
const emit = defineEmits(["close", "done"]);
const open = defineModel("open", { type: Boolean, default: false });

const toasts = useToasts();
const structure = ref(true);
const data = ref(true);
const picked = ref([]);
const limit = ref("");
const running = ref(false);
const progress = ref(null);
const failed = ref("");
/** Where the file will be written, shown before anything starts. */
const destination = ref("");
/** "all 12 tables and their rows", said back before it starts. */
const chosenLabel = computed(() => {
  const tables = picked.value.length
    ? picked.value.length
    : props.available.length;
  const noun = tables === 1 ? "table" : "tables";
  if (structure.value && data.value)
    return `all ${tables} ${noun} and their rows`;
  if (data.value) return `the rows of ${tables} ${noun}`;
  return `the structure of ${tables} ${noun}`;
});

const limitRows = computed(() => {
  const value = Number.parseInt(limit.value, 10);
  return Number.isFinite(value) && value > 0 ? value : null;
});

watch(
  () => props.open,
  async (isOpen) => {
    if (!isOpen) return;
    structure.value = true;
    data.value = true;
    limit.value = "";
    failed.value = "";
    progress.value = null;
    picked.value = [...(props.tables.length ? props.tables : [])];
    // Shown, not asked for: the question "where will this go" should be
    // answered on screen rather than by a file picker that appears after the
    // decision has already been made.
    destination.value = await defaultBackupPath(props.database);
  },
);

function toggle(name) {
  picked.value = picked.value.includes(name)
    ? picked.value.filter((one) => one !== name)
    : [...picked.value, name];
}

function all() {
  picked.value = [...props.available];
}
function none() {
  picked.value = [];
}

/** Where the backup goes. Changing it does not start anything. */
async function choose() {
  const picked = await pickSqlDestination(destination.value);
  if (picked) destination.value = picked;
}

async function start() {
  if (!structure.value && !data.value) {
    failed.value = "Choose structure, data, or both.";
    return;
  }
  // The path shown on the panel, not a new question. Opening a picker here
  // asked where the file should go *after* the panel had already answered it.
  const path = destination.value;
  if (!path) {
    failed.value = "Choose where to save the backup first.";
    return;
  }

  running.value = true;
  failed.value = "";
  try {
    const summary = await exportSql(props.profileId, {
      database: props.database,
      schema: props.schema || null,
      path,
      options: {
        structure: structure.value,
        data: data.value,
        tables: picked.value,
        rowLimit: limitRows.value,
      },
    });
    toasts.success(
      "Export complete",
      `${summary.tables} tables, ${summary.rows.toLocaleString()} rows, ${formatBytes(summary.bytes)}.`,
    );
    emit("done", summary);
    open.value = false;
  } catch (failure) {
    // A cancelled export leaves a partial file, which is not a file anyone
    // should find later and mistake for a backup.
    failed.value = toUserDetail(failure);
  } finally {
    running.value = false;
    progress.value = null;
  }
}

async function cancel() {
  await cancelSqlExport(props.profileId);
  failed.value = "Stopping after the current page…";
}

function formatBytes(bytes) {
  if (!bytes) return "0 bytes";
  const units = ["bytes", "KB", "MB", "GB"];
  const power = Math.min(
    units.length - 1,
    Math.floor(Math.log(bytes) / Math.log(1024)),
  );
  const value = bytes / 1024 ** power;
  return `${value.toFixed(power ? 1 : 0)} ${units[power]}`;
}

/**
 * Listens for the dump's progress while the dialog is open.
 *
 * Registered once on mount rather than per export: a listener left behind
 * after the dialog closes keeps its closure, and everything it captured, alive
 * for the life of the window.
 */
let stop = null;
onMounted(async () => {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    stop = await listen("sql-export-progress", (event) => {
      progress.value = event.payload;
    });
  } catch {
    // No event bridge (the mock preview): the export still works, it just does
    // not say how far along it is.
  }
});
onBeforeUnmount(() => {
  stop?.();
  stop = null;
});
</script>

<template>
  <BaseDialog
    v-model:open="open"
    title="Create Backup"
    size="lg"
    @close="emit('close')"
  >
    <div class="space-y-4">
      <!-- Confirms what was asked for, in the terms it was asked in. -->
      <p v-if="!running && !failed" class="text-xs text-fg-subtle">
        {{ database }}{{ schema ? ` / ${schema}` : "" }} — {{ chosenLabel }}
        will be written to the folder above.
      </p>

      <InlineAlert v-if="failed" tone="danger">
        {{ failed }}
        <p class="mt-1 text-xs opacity-80">
          The file was left where it was, incomplete. Delete it or overwrite it.
        </p>
      </InlineAlert>

      <BackupForm
        v-if="!running"
        :destination="destination"
        :database="database"
        :schema="schema"
        :engine="engine"
        :structure="structure"
        :rows="data"
        :limit="limit"
        :available="available"
        :picked="picked"
        @change-destination="choose"
        @update:structure="structure = $event"
        @update:rows="data = $event"
        @update:limit="limit = $event"
        @toggle-table="toggle"
        @all-tables="all"
        @no-tables="none"
      />

      <ExportProgress
        v-else
        :progress="progress"
        :where="schema || database"
        @stop="cancel"
      />
    </div>

    <template #footer>
      <template v-if="!running">
        <BaseButton variant="ghost" @click="open = false">Cancel</BaseButton>
        <BaseButton variant="primary" @click="start">
          <FileCode2 :size="14" :stroke-width="1.75" aria-hidden="true" />
          Create Backup
        </BaseButton>
      </template>
    </template>
  </BaseDialog>
</template>

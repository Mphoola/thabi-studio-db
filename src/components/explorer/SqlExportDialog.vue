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
import CheckboxInput from "../common/CheckboxInput.vue";
import ExportProgress from "./ExportProgress.vue";
import ExportTablePicker from "./ExportTablePicker.vue";
import InlineAlert from "../common/InlineAlert.vue";
import TextInput from "../common/TextInput.vue";
import { useToasts } from "../../composables/useToasts.js";
import { cancelSqlExport, exportSql } from "../../lib/tauri/commands.js";
import { pickSqlDestination } from "../../lib/tauri/files.js";
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

/** MySQL will state its own CREATE TABLE; the other two cannot. */
const rebuilt = computed(
  () => props.engine === "postgres" || props.engine === "sqlserver",
);
const limitRows = computed(() => {
  const value = Number.parseInt(limit.value, 10);
  return Number.isFinite(value) && value > 0 ? value : null;
});

watch(
  () => props.open,
  (isOpen) => {
    if (!isOpen) return;
    structure.value = true;
    data.value = true;
    limit.value = "";
    failed.value = "";
    progress.value = null;
    picked.value = [...(props.tables.length ? props.tables : [])];
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

async function start() {
  if (!structure.value && !data.value) {
    failed.value = "Choose structure, data, or both.";
    return;
  }
  const path = await pickSqlDestination(
    `${props.database}-${new Date().toISOString().slice(0, 10)}`,
  );
  if (!path) return;

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
    title="Export as SQL"
    size="lg"
    @close="emit('close')"
  >
    <div class="space-y-4">
      <InlineAlert v-if="failed" tone="danger">
        {{ failed }}
        <p class="mt-1 text-xs opacity-80">
          The file was left where it was, incomplete. Delete it or overwrite it.
        </p>
      </InlineAlert>

      <template v-if="!running">
        <section>
          <h3 class="mb-1.5 text-xs font-semibold text-fg-muted">Contents</h3>
          <div class="flex gap-4">
            <CheckboxInput v-model="structure" label="Table structure" />
            <CheckboxInput v-model="data" label="Rows" />
          </div>
          <p v-if="rebuilt" class="mt-1.5 text-xs text-fg-subtle">
            {{ engine === "postgres" ? "PostgreSQL" : "SQL Server" }} cannot
            print its own <code>CREATE TABLE</code>, so the structure here is
            rebuilt from the catalog. Columns, types, keys, indexes and
            single-column foreign keys are included; check constraints, triggers
            and partitions are not. Use
            <code>{{ engine === "postgres" ? "pg_dump" : "sqlpackage" }}</code>
            for those.
          </p>
          <p v-else class="mt-1.5 text-xs text-fg-subtle">
            The structure is the server's own <code>CREATE TABLE</code>, so it
            is exact.
          </p>
        </section>

        <ExportTablePicker
          v-if="available.length > 1"
          :available="available"
          :picked="picked"
          @toggle="toggle"
          @all="all"
          @none="none"
        />

        <section>
          <TextInput
            v-model="limit"
            label="Row limit, per table (optional)"
            placeholder="all rows"
            inputmode="numeric"
          />
          <p class="mt-1 text-xs text-fg-subtle">
            Leave this empty. A limit makes the file a sample, not a backup.
          </p>
        </section>

        <InlineAlert tone="warning">
          Every table in the file is preceded by
          <code>DROP TABLE IF EXISTS</code>. Running this file over a database
          that already has these tables replaces them.
        </InlineAlert>
      </template>

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
          Export…
        </BaseButton>
      </template>
    </template>
  </BaseDialog>
</template>

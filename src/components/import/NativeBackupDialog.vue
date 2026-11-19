<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { FolderOpen } from "lucide-vue-next";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { toUserDetail } from "../../lib/errors.js";
import { formatBytes } from "../../lib/formatSize.js";
import {
  backupDatabase,
  backupLog,
  backupTools,
  cancelBackup,
  restoreDatabase,
} from "../../lib/tauri/commands.js";
import {
  pickBackupDestination,
  pickBackupFile,
} from "../../lib/tauri/files.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import InlineAlert from "../common/InlineAlert.vue";
import TextInput from "../common/TextInput.vue";

/**
 * Backs a database up, or restores one, with the engine's own tools:
 * pg_dump / pg_restore, mysqldump / mysql, or SQL Server's BACKUP and
 * RESTORE, which use a path on the server. The tool's own log is shown as it
 * runs. A restore replaces the database, so its name is typed first.
 */
const open = defineModel("open", { type: Boolean, default: false });
const props = defineProps({
  /** { profileId, database, mode: "backup" | "restore" } */
  target: { type: Object, required: true },
});

const connections = useConnectionProfiles();
const profile = computed(() =>
  connections.profiles.value.find((p) => p.id === props.target.profileId),
);
const restoring = computed(() => props.target.mode === "restore");
const jobId = `backup:${props.target.profileId}:${props.target.database}`;
const tools = ref(null);
const path = ref("");
const typed = ref("");
const running = ref(false);
const log = ref([]);
const result = ref(null);
const error = ref("");
const logBox = ref(null);
let timer = null;

const serverSide = computed(() => tools.value?.serverSide);
const tool = computed(() => {
  if (!tools.value || serverSide.value) return null;
  return restoring.value
    ? tools.value.archiveRestore || tools.value.restore
    : tools.value.dump;
});
// The version is picked when it runs, to suit the server; the log names it.
const toolName = computed(() => tool.value?.split(/[\\/]/).pop() ?? "");
const extension = computed(() =>
  profile.value?.driver === "postgres" ? "dump" : "sql",
);
const ready = computed(
  () =>
    Boolean(path.value.trim()) &&
    (serverSide.value || Boolean(tool.value)) &&
    (!restoring.value || typed.value.trim() === props.target.database),
);

onMounted(async () => {
  try {
    tools.value = await backupTools(props.target.profileId);
    if (tools.value.serverSide)
      path.value = `/var/opt/mssql/data/${props.target.database}.bak`;
  } catch (failure) {
    error.value = toUserDetail(failure);
  }
});

async function choose() {
  const picked = restoring.value
    ? await pickBackupFile()
    : await pickBackupDestination(props.target.database, extension.value);
  if (picked) path.value = picked;
}

async function run() {
  running.value = true;
  error.value = "";
  result.value = null;
  timer = setInterval(async () => {
    log.value = await backupLog(jobId).catch(() => log.value);
    await nextTick();
    logBox.value?.scrollTo({ top: logBox.value.scrollHeight });
  }, 500);
  try {
    const call = restoring.value ? restoreDatabase : backupDatabase;
    result.value = await call(
      props.target.profileId,
      jobId,
      props.target.database,
      path.value.trim(),
    );
    log.value = result.value.log;
  } catch (failure) {
    error.value = toUserDetail(failure);
  } finally {
    clearInterval(timer);
    running.value = false;
  }
}

onBeforeUnmount(() => clearInterval(timer));
</script>

<template>
  <BaseDialog
    v-model:open="open"
    size="lg"
    :title="`${restoring ? 'Restore' : 'Back Up'} “${target.database}”`"
  >
    <div class="flex flex-col gap-3">
      <template v-if="!running && !result">
        <p v-if="serverSide" class="text-sm text-fg-muted">
          SQL Server {{ restoring ? "reads" : "writes" }} the backup itself, so
          the path is on the server, not on this computer.
        </p>
        <p v-else-if="tools && tool" class="text-sm text-fg-muted">
          Uses <code class="font-mono text-xs">{{ toolName }}</code
          >, the version that suits this server, with the connection's login.
          The password is passed to it privately, never on its command line.
        </p>
        <InlineAlert
          v-else-if="tools"
          tone="warning"
          title="The database's client tools aren't installed"
          :message="`Install ${profile?.driver === 'postgres' ? 'PostgreSQL' : 'MySQL'}'s client tools (for example with Homebrew), then open this again.`"
        />
        <TextInput
          v-if="serverSide"
          v-model="path"
          mono
          label="Path on the server"
        />
        <div v-else class="flex items-center gap-3">
          <BaseButton :icon="FolderOpen" @click="choose">{{
            restoring ? "Choose Backup…" : "Choose Where…"
          }}</BaseButton>
          <span class="truncate font-mono text-xs text-fg-muted">{{
            path || "No file chosen"
          }}</span>
        </div>
        <template v-if="restoring">
          <InlineAlert
            tone="danger"
            title="A restore replaces the database"
            :message="
              serverSide
                ? 'Other connections to it are closed first, and everything in it is replaced by the backup.'
                : 'Tables in the backup are dropped and made again from it.'
            "
          />
          <TextInput
            v-model="typed"
            mono
            :label="`Type ${target.database} to continue`"
            autocomplete="off"
          />
        </template>
      </template>
      <pre
        v-if="running || log.length"
        ref="logBox"
        class="max-h-64 overflow-auto rounded-md bg-surface-hover p-2 font-mono text-xs whitespace-pre-wrap text-fg-muted"
        >{{ log.join("\n") || "Starting…" }}</pre>
      <InlineAlert
        v-if="error"
        tone="danger"
        :title="`The ${restoring ? 'restore' : 'backup'} didn't finish`"
        :message="error"
      />
      <template v-if="result">
        <InlineAlert
          tone="success"
          :title="
            restoring
              ? `Restored “${target.database}”.`
              : `Backed up to ${result.path}${result.bytes ? ` (${formatBytes(result.bytes)})` : ''}.`
          "
        />
        <InlineAlert
          v-for="warning in result.warnings"
          :key="warning"
          tone="warning"
          title="Not everything is in this backup"
          :message="warning"
        />
      </template>
    </div>
    <template #footer>
      <BaseButton v-if="running" @click="cancelBackup(jobId)">Stop</BaseButton>
      <BaseButton
        v-else-if="result || error"
        variant="primary"
        @click="open = false"
        >Done</BaseButton
      >
      <template v-else>
        <BaseButton @click="open = false">Cancel</BaseButton>
        <BaseButton
          :variant="restoring ? 'danger' : 'primary'"
          :disabled="!ready"
          @click="run"
          >{{ restoring ? "Restore" : "Back Up" }}</BaseButton
        >
      </template>
    </template>
  </BaseDialog>
</template>

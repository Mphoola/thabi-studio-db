<script setup>
import { computed, onBeforeUnmount, ref } from "vue";
import { FileCode2 } from "lucide-vue-next";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { toUserDetail } from "../../lib/errors.js";
import {
  cancelStatement,
  runSqlFile,
  sqlFileProgress,
} from "../../lib/tauri/commands.js";
import { pickSqlFile } from "../../lib/tauri/files.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import InlineAlert from "../common/InlineAlert.vue";
import SegmentedControl from "../common/SegmentedControl.vue";
import TextInput from "../common/TextInput.vue";

/**
 * Runs a SQL dump into a database, such as a backup made here: statement by
 * statement on a tab's own session, with progress, stopping at the first
 * error or carrying on and listing each one by its line. Statements that ran
 * stay run; a dump is not one transaction.
 */
const open = defineModel("open", { type: Boolean, default: false });
const props = defineProps({
  /** { profileId, database } */
  target: { type: Object, required: true },
});
const emit = defineEmits(["done"]);

const connections = useConnectionProfiles();
const profile = computed(() =>
  connections.profiles.value.find((p) => p.id === props.target.profileId),
);
const tabId = `restore:${props.target.database}`;
const path = ref("");
const onError = ref("stop");
const typed = ref("");
const running = ref(false);
const progress = ref(null);
const summary = ref(null);
const error = ref("");
let timer = null;

const fileName = computed(() => path.value.split(/[\\/]/).pop());
// Production asks for the database's name typed: a restore overwrites.
const needsName = computed(() => profile.value?.environment === "production");
const ready = computed(
  () =>
    Boolean(path.value) &&
    (!needsName.value || typed.value.trim() === props.target.database),
);
const percent = computed(() => {
  const [done, total] = progress.value ?? [0, 0];
  return total ? Math.round((done / total) * 100) : 0;
});

async function choose() {
  const picked = await pickSqlFile("Choose a SQL Dump");
  if (picked) path.value = picked;
}

async function run() {
  running.value = true;
  error.value = "";
  summary.value = null;
  timer = setInterval(async () => {
    progress.value =
      (await sqlFileProgress(props.target.profileId, tabId).catch(
        () => null,
      )) ?? progress.value;
  }, 400);
  try {
    summary.value = await runSqlFile(props.target.profileId, tabId, {
      database: props.target.database,
      path: path.value,
      keepGoing: onError.value === "continue",
    });
    emit("done");
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
    size="md"
    :title="`Run a SQL Dump into “${target.database}”`"
  >
    <div class="flex flex-col gap-3">
      <template v-if="!running && !summary">
        <div class="flex items-center gap-3">
          <BaseButton :icon="FileCode2" @click="choose"
            >Choose File…</BaseButton
          >
          <span class="truncate font-mono text-xs text-fg-muted">{{
            fileName || "No file chosen"
          }}</span>
        </div>
        <SegmentedControl
          v-model="onError"
          label="On an error"
          :options="[
            { value: 'stop', label: 'Stop at the first error' },
            { value: 'continue', label: 'Continue and list errors' },
          ]"
        />
        <p class="text-sm text-fg-muted">
          Runs on “{{ profile?.name }}”, statement by statement. Statements that
          ran stay run, even if a later one fails.
        </p>
        <TextInput
          v-if="needsName"
          v-model="typed"
          mono
          :label="`Production: type ${target.database} to continue`"
          autocomplete="off"
        />
      </template>
      <div v-if="running" class="flex flex-col gap-2" role="status">
        <div class="h-2 rounded-full bg-surface-hover">
          <div
            class="h-2 rounded-full bg-accent transition-[width]"
            :style="{ width: `${percent}%` }"
          />
        </div>
        <p class="text-sm text-fg-muted">
          {{ progress?.[0] ?? 0 }} of {{ progress?.[1] ?? "…" }} statements
        </p>
      </div>
      <InlineAlert
        v-if="error"
        tone="danger"
        title="The file couldn't be run"
        :message="error"
      />
      <template v-if="summary">
        <InlineAlert
          :tone="summary.errors.length ? 'warning' : 'success'"
          :title="
            summary.cancelled
              ? `Stopped after ${summary.ran} of ${summary.total} statements.`
              : `Ran ${summary.ran} of ${summary.total} statements${summary.errors.length ? `, ${summary.errors.length} failed` : ''}.`
          "
        />
        <ul
          v-if="summary.errors.length"
          class="max-h-56 overflow-auto rounded-md border border-line text-xs"
        >
          <li
            v-for="(failure, i) in summary.errors"
            :key="i"
            class="border-b border-line px-2 py-1 last:border-0"
          >
            <span class="font-mono text-fg-subtle"
              >Line {{ failure.line }}</span
            >
            {{ failure.message }}
            <span v-if="failure.detail" class="block text-fg-muted">{{
              failure.detail
            }}</span>
          </li>
        </ul>
      </template>
    </div>
    <template #footer>
      <template v-if="running">
        <BaseButton @click="cancelStatement(target.profileId, tabId)"
          >Stop</BaseButton
        >
      </template>
      <template v-else-if="summary || error">
        <BaseButton variant="primary" @click="open = false">Done</BaseButton>
      </template>
      <template v-else>
        <BaseButton @click="open = false">Cancel</BaseButton>
        <BaseButton
          :variant="needsName ? 'danger' : 'primary'"
          :disabled="!ready"
          @click="run"
          >Run Dump</BaseButton
        >
      </template>
    </template>
  </BaseDialog>
</template>

<script setup>
import { computed, ref } from "vue";
import { CircleAlert, CircleCheck, CircleSlash, Clock } from "lucide-vue-next";
import InlineAlert from "../common/InlineAlert.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";
import ResultView from "../grid/ResultView.vue";
import ChangesPreviewDialog from "../explorer/ChangesPreviewDialog.vue";
import EditChangesBar from "../explorer/EditChangesBar.vue";
import { useTableEditing } from "../explorer/useTableEditing.js";
import { previewSource, remoteSource } from "../../lib/gridSources.js";
import { useResultEditability } from "./useResultEditability.js";
import ReconnectButton from "../connections/ReconnectButton.vue";
import PlanView from "./PlanView.vue";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";

/** Results of the last run: one tab per statement, plus messages. */
const props = defineProps({ tab: { type: Object, required: true } });

const run = computed(() => props.tab.runs[props.tab.activeRun] ?? null);
// The table a single-table SELECT reads, for SQL copy/export formats.
const resultTarget = ref(null);
// A cancelled or timed-out run still shows the rows that had arrived.
const source = computed(() =>
  ["completed", "cancelled", "timed_out"].includes(run.value?.status) &&
  run.value.result?.columns?.length
    ? remoteSource({
        profileId: props.tab.profileId,
        tabId: props.tab.id,
        result: run.value.result,
        exportName: props.tab.title,
        target: () => resultTarget.value,
      })
    : null,
);
const resultColumns = computed(() => run.value?.result?.columns ?? []);
const { editNode, whyNotEditable } = useResultEditability({
  tab: props.tab,
  run,
  resultTarget,
});

const editing = useTableEditing({
  tab: props.tab,
  node: editNode,
  profileId: computed(() => props.tab.profileId),
  columns: resultColumns,
  reload: () => props.tab.rerunQuery?.(),
});
const editor = editing.editor;

const store = useConnectionProfiles();
// Errors name where the query ran, e.g. Query failed on “Production” (shop).
const failedTitle = computed(() => {
  const name = store.profiles.value.find(
    (p) => p.id === props.tab.profileId,
  )?.name;
  const where = props.tab.database ? ` (${props.tab.database})` : "";
  return name ? `Query failed on “${name}”${where}` : "Query failed";
});
function failureDetail(error) {
  const parts =
    error?.kind === "server"
      ? [error.detail, error.code && `Code ${error.code}`]
      : [error?.message, error?.detail];
  return parts.filter(Boolean).join(" · ");
}
const icons = {
  completed: CircleCheck,
  failed: CircleAlert,
  cancelled: CircleSlash,
  timed_out: Clock,
};
const tones = {
  completed: "text-success-text",
  failed: "text-danger-text",
  cancelled: "text-fg-muted",
  timed_out: "text-warning-text",
};

function label(entry, index) {
  if (entry.kind === "plan") return entry.analyzed ? "Plan (analyzed)" : "Plan";
  if (entry.kind === "script") return entry.file;
  if (entry.status === "running") return `Statement ${index + 1}`;
  if (entry.status !== "completed") return `Statement ${index + 1}`;
  const r = entry.result;
  if (r.columns.length) {
    const total = (r.totalRows ?? r.rows.length).toLocaleString();
    return `Result ${index + 1} · ${total}${r.truncated ? "+" : ""} rows`;
  }
  return `Statement ${index + 1}`;
}

function message(entry) {
  if (entry.kind === "script") {
    const { ran, total, rowsAffected } = entry.script;
    return `Ran ${ran.toLocaleString()} of ${total.toLocaleString()} statements · ${rowsAffected.toLocaleString()} rows affected`;
  }
  const r = entry.result;
  if (!r) return "";
  const time = `${r.elapsedMs.toLocaleString()} ms`;
  if (r.rowsAffected !== null && r.rowsAffected !== undefined)
    return `${r.rowsAffected.toLocaleString()} ${r.rowsAffected === 1 ? "row" : "rows"} affected · ${time}`;
  return `Query ran. No rows returned. · ${time}`;
}
</script>

<template>
  <div class="flex min-h-0 flex-col bg-canvas">
    <div
      v-if="tab.runs.length"
      role="tablist"
      aria-label="Results"
      class="flex h-8 shrink-0 items-center gap-1 overflow-x-auto border-b border-line bg-toolbar px-2"
    >
      <button
        v-for="(entry, index) in tab.runs"
        :key="index"
        type="button"
        role="tab"
        :aria-selected="tab.activeRun === index"
        class="flex shrink-0 items-center gap-1.5 rounded-md px-2.5 py-0.5 text-sm"
        :class="
          tab.activeRun === index
            ? 'bg-surface text-fg shadow-[0_0_0_1px_var(--line)]'
            : 'text-fg-muted hover:text-fg'
        "
        @click="tab.activeRun = index"
      >
        <SpinnerIcon v-if="entry.status === 'running'" :size="10" />
        <component
          :is="icons[entry.status]"
          v-else
          :size="12"
          :stroke-width="2"
          :class="tones[entry.status]"
          aria-hidden="true"
        />
        {{ label(entry, index) }}
      </button>
    </div>

    <div
      v-if="!run"
      class="flex flex-1 items-center justify-center text-sm text-fg-subtle"
    >
      Run a statement to see results here.
    </div>
    <template v-else-if="run.status === 'running'">
      <ResultView
        v-if="run.progress?.preview?.columns?.length"
        class="min-h-0 flex-1"
        :source="previewSource(run.progress.preview)"
        :result="run.progress.preview"
        :note="`Still running · ${run.progress.received.toLocaleString()} rows so far`"
      />
      <div
        v-else
        class="flex flex-1 items-center justify-center gap-2 text-fg-subtle"
      >
        <SpinnerIcon :size="14" />
        {{
          run.kind === "script" && run.scriptProgress
            ? `Running file… ${run.scriptProgress[0].toLocaleString()} of ${run.scriptProgress[1].toLocaleString()} statements`
            : "Running…"
        }}
      </div>
    </template>
    <PlanView
      v-else-if="run.kind === 'plan' && run.plan"
      class="min-h-0 flex-1"
      :plan="run.plan"
    />
    <template v-else-if="source">
      <EditChangesBar
        v-if="editor.summary.value.total"
        :summary="editor.summary.value"
        :applying="editing.applying.value"
        @preview="editing.showPreview"
        @discard="editing.discard"
        @apply="editing.apply"
      />
      <ResultView
        class="min-h-0 flex-1"
        :source="source"
        :result="run.result"
        :editor="editNode && run.status === 'completed' ? editor : null"
        :note="
          run.status !== 'completed'
            ? `${run.error?.message ?? 'Stopped.'} These are the rows that arrived first.`
            : editNode
              ? editing.readOnlyReason.value ||
                `Editing as ${editNode.name}. Double-click a cell to edit.`
              : whyNotEditable
        "
      />
    </template>
    <div v-else class="flex flex-col gap-3 overflow-auto p-4">
      <InlineAlert
        v-if="run.status === 'completed'"
        tone="success"
        :title="message(run)"
      />
      <InlineAlert
        v-else-if="run.status === 'cancelled' || run.status === 'timed_out'"
        tone="warning"
        :title="run.error?.message ?? 'Query stopped.'"
      />
      <InlineAlert
        v-else
        tone="danger"
        :title="failedTitle"
        :message="failureDetail(run.error)"
        mono
      >
        <ReconnectButton
          v-if="['connection_lost', 'not_connected'].includes(run.error?.kind)"
          :profile-id="tab.profileId"
        />
      </InlineAlert>
      <pre
        class="selectable max-h-40 overflow-auto rounded-md bg-surface-hover p-3 font-mono text-sm whitespace-pre-wrap text-fg-muted"
        >{{ run.sql }}</pre>
    </div>
    <ChangesPreviewDialog
      :statements="editing.preview.value"
      :applying="editing.applying.value"
      @close="editing.preview.value = null"
      @apply="editing.apply"
    />
  </div>
</template>

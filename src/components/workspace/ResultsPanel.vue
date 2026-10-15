<script setup>
import { computed } from "vue";
import { CircleAlert, CircleCheck, CircleSlash, Clock } from "lucide-vue-next";
import InlineAlert from "../common/InlineAlert.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";
import ResultView from "../grid/ResultView.vue";
import { remoteSource } from "../../lib/gridSources.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";

/** Results of the last run: one tab per statement, plus messages. */
const props = defineProps({ tab: { type: Object, required: true } });

const run = computed(() => props.tab.runs[props.tab.activeRun] ?? null);
const source = computed(() =>
  run.value?.status === "completed" && run.value.result.columns.length
    ? remoteSource({
        profileId: props.tab.profileId,
        tabId: props.tab.id,
        result: run.value.result,
        exportName: props.tab.title,
      })
    : null,
);
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
    <div
      v-else-if="run.status === 'running'"
      class="flex flex-1 items-center justify-center gap-2 text-fg-subtle"
    >
      <SpinnerIcon :size="14" /> Running…
    </div>
    <ResultView
      v-else-if="source"
      class="min-h-0 flex-1"
      :source="source"
      :result="run.result"
    />
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
      />
      <pre
        class="selectable max-h-40 overflow-auto rounded-md bg-surface-hover p-3 font-mono text-sm whitespace-pre-wrap text-fg-muted"
        >{{ run.sql }}</pre>
    </div>
  </div>
</template>

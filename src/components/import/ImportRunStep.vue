<script setup>
import { computed } from "vue";
import InlineAlert from "../common/InlineAlert.vue";
import { formatBytes } from "../../lib/formatSize.js";

/** The import running, then what happened, with refused rows by line. */
const props = defineProps({ state: { type: Object, required: true } });
const s = props.state;

const percent = computed(() => {
  const status = s.status.value;
  if (!status?.totalBytes) return 0;
  return Math.min(100, Math.round((status.bytes / status.totalBytes) * 100));
});
const rows = (n) => `${n.toLocaleString()} ${n === 1 ? "row" : "rows"}`;
const outcome = computed(() => {
  const summary = s.summary.value;
  if (!summary) return null;
  if (summary.cancelled)
    return { tone: "warning", title: "Stopped. Nothing was imported." };
  if (summary.rolledBack)
    return {
      tone: "danger",
      title: "A row was refused, so nothing was imported.",
    };
  if (summary.rejectedTotal)
    return {
      tone: "warning",
      title: `Imported ${rows(summary.inserted)}. ${rows(summary.rejectedTotal)} ${summary.rejectedTotal === 1 ? "was" : "were"} skipped.`,
    };
  return {
    tone: "success",
    title: `Imported ${rows(summary.inserted)}.`,
  };
});
</script>

<template>
  <div class="flex flex-col gap-3">
    <div v-if="s.running.value" class="flex flex-col gap-2" role="status">
      <div class="h-2 rounded-full bg-surface-hover">
        <div
          class="h-2 rounded-full bg-accent transition-[width]"
          :style="{ width: `${percent}%` }"
        />
      </div>
      <p class="text-sm text-fg-muted">
        {{ (s.status.value?.rows ?? 0).toLocaleString() }} rows so far ·
        {{ formatBytes(s.status.value?.bytes ?? 0) }} of
        {{ formatBytes(s.status.value?.totalBytes ?? 0) }}
        <template v-if="s.status.value?.rejected">
          · {{ s.status.value.rejected }} refused</template
        >
      </p>
    </div>
    <InlineAlert
      v-if="s.runError.value"
      tone="danger"
      title="The import failed, and nothing was kept"
      :message="s.runError.value"
    />
    <InlineAlert v-if="outcome" :tone="outcome.tone" :title="outcome.title" />
    <div
      v-if="s.summary.value?.rejected.length"
      class="max-h-56 overflow-auto rounded-md border border-line"
    >
      <table class="w-full text-xs">
        <thead class="sticky top-0 bg-grid-header text-left text-fg-muted">
          <tr>
            <th class="w-20 px-2 py-1 font-semibold">Line</th>
            <th class="px-2 py-1 font-semibold">Why</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="row in s.summary.value.rejected"
            :key="row.line"
            class="border-t border-line"
          >
            <td class="px-2 py-1 font-mono tabular-nums">{{ row.line }}</td>
            <td class="px-2 py-1">{{ row.message }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

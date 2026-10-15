<script setup>
import { computed } from "vue";
import { Download } from "lucide-vue-next";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail } from "../../lib/errors.js";
import { showInFinder } from "../../lib/tauri/files.js";
import BaseButton from "../common/BaseButton.vue";
import ResultGrid from "./ResultGrid.vue";

/** A result grid with its summary bar: rows, time, limits and export. */
const props = defineProps({
  source: { type: Object, required: true },
  result: { type: Object, required: true },
  /** Extra note, e.g. "Read-only preview". */
  note: { type: String, default: "" },
});
const toasts = useToasts();

const summary = computed(() => {
  const total = props.source.total.toLocaleString();
  const rows = props.source.total === 1 ? "1 row" : `${total} rows`;
  return props.result.truncated ? `First ${total} rows fetched` : rows;
});

async function exportCsv() {
  try {
    const done = await props.source.export({ includeHeader: true }, null);
    if (!done) return;
    const file = done.path.split("/").pop();
    toasts.success(
      `Exported ${done.rows.toLocaleString()} rows to ${file}`,
      "",
      {
        label: "Show in Finder",
        run: () => showInFinder(done.path),
      },
    );
  } catch (error) {
    toasts.error("Couldn’t export", toUserDetail(error));
  }
}
</script>

<template>
  <div class="flex min-h-0 flex-col">
    <ResultGrid :source="source" class="min-h-0 flex-1" />
    <footer
      class="flex h-7 shrink-0 items-center gap-2 border-t border-line bg-toolbar px-3 text-xs text-fg-muted"
    >
      <span>{{ summary }}</span>
      <span>· {{ result.elapsedMs.toLocaleString() }} ms</span>
      <span v-if="result.truncated" class="text-warning-text">
        · More rows exist. Add a WHERE or LIMIT, or raise the limit in Settings.
      </span>
      <span v-else-if="result.extraResultSets">
        · {{ result.extraResultSets }} more result sets not shown
      </span>
      <span v-if="note">· {{ note }}</span>
      <BaseButton
        v-if="source.export"
        class="ml-auto"
        size="sm"
        variant="ghost"
        :icon="Download"
        @click="exportCsv"
      >
        Export CSV…
      </BaseButton>
    </footer>
  </div>
</template>

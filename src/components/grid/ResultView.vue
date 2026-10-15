<script setup>
import { computed, ref } from "vue";
import { Download } from "lucide-vue-next";
import { EXPORT_FORMATS } from "../../lib/tauri/files.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail } from "../../lib/errors.js";
import { showInFileManager } from "../../lib/tauri/files.js";
import { words } from "../../lib/platform.js";
import BaseButton from "../common/BaseButton.vue";
import DropdownMenu from "../common/DropdownMenu.vue";
import ResultGrid from "./ResultGrid.vue";

/** A result grid with its summary bar: rows, time, limits and export. */
const props = defineProps({
  source: { type: Object, required: true },
  result: { type: Object, required: true },
  /** Extra note, e.g. "Read-only preview". */
  note: { type: String, default: "" },
  /** Staged row edits (useRowEdits); makes the grid editable. */
  editor: { type: Object, default: null },
});
const toasts = useToasts();

const summary = computed(() => {
  const total = props.source.total.toLocaleString();
  const rows = props.source.total === 1 ? "1 row" : `${total} rows`;
  return props.result.truncated ? `First ${total} rows fetched` : rows;
});

/** Format for the export; null keeps the source's own default (CSV). */
const format = ref(null);
const exportMenu = ref(null);
const formatItems = EXPORT_FORMATS.map((entry) => ({
  id: entry.value,
  label: entry.label,
}));

function chooseFormat(value) {
  format.value = value;
  exportCsv();
}

async function exportCsv() {
  try {
    const done = await props.source.export(
      { includeHeader: true },
      null,
      format.value,
    );
    if (!done) return;
    const file = done.path.split("/").pop();
    toasts.success(
      `Exported ${done.rows.toLocaleString()} rows to ${file}`,
      "",
      {
        label: words.fileManager,
        run: () => showInFileManager(done.path),
      },
    );
  } catch (error) {
    toasts.error("Couldn’t export", toUserDetail(error));
  }
}
</script>

<template>
  <div class="flex min-h-0 flex-col">
    <ResultGrid :source="source" :editor="editor" class="min-h-0 flex-1" />
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
        @click="(event) => exportMenu?.openAt(event.clientX, event.clientY)"
      >
        Export…
      </BaseButton>
      <DropdownMenu
        ref="exportMenu"
        :items="formatItems"
        label="Export format"
        @select="chooseFormat"
      />
    </footer>
  </div>
</template>

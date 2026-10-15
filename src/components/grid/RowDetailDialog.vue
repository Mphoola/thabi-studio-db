<script setup>
import { computed, nextTick, ref, watch } from "vue";
import { Copy } from "lucide-vue-next";
import { copyText } from "../../lib/clipboard.js";
import { useToasts } from "../../composables/useToasts.js";
import { inspectText } from "../../lib/gridText.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";

/**
 * One row as a vertical form: every column, its type, and the value in full.
 *
 * Where the grid is for comparing rows at a glance, this is for reading one
 * properly — long text and JSON wrapped out rather than clipped. When the
 * table can be edited, each value is edited here through the same staged
 * changes the grid uses, so the two never disagree about what is pending.
 */
const open = defineModel("open", { type: Boolean, default: false });
const props = defineProps({
  columns: { type: Array, required: true },
  /** Cells of the focused row, in source column order. */
  cells: { type: Array, default: null },
  /** Position of the row in the grid, for the title. */
  row: { type: Number, default: 0 },
  /** useRowEdits, when the table can be edited. */
  editor: { type: Object, default: null },
});
const toasts = useToasts();

/** The column being edited here, or -1. */
const editing = ref(-1);
const draft = ref("");
const box = ref(null);

const title = computed(() =>
  props.row > 0 ? `Row ${props.row.toLocaleString()}` : "Row",
);

/** What a column shows: a staged edit when there is one. */
function shown(index) {
  if (!props.cells) return null;
  return props.editor
    ? props.editor.display(props.cells, index)
    : props.cells[index];
}

/** The full text of a value, for wrapping out instead of clipping. */
function fullText(index) {
  const cell = shown(index);
  if (!cell) return "";
  return inspectText(cell, props.columns[index]?.typeName) ?? "";
}

const editable = computed(
  () => props.editor?.enabled.value && !props.editor.isDeleted(props.cells),
);

watch(open, async (isOpen) => {
  editing.value = -1;
  if (isOpen) await nextTick();
});

async function startEdit(index) {
  if (!editable.value || !props.editor?.canEdit(props.cells, index)) return;
  draft.value = props.editor.valueOf(props.cells, index) ?? "";
  editing.value = index;
  await nextTick();
  box.value?.focus();
}

function commit(move = null) {
  const index = editing.value;
  if (index < 0) return;
  props.editor.setValue(props.cells, index, draft.value);
  editing.value = -1;
  if (move !== null && editable.value) startEdit(move);
}

function cancel() {
  editing.value = -1;
}

async function copyRow() {
  const lines = props.columns.map((column, index) => {
    const cell = shown(index);
    const text = cell?.kind === "null" ? "NULL" : (cell?.value ?? "");
    return `${column.name}\t${text}`;
  });
  try {
    await copyText(lines.join("\n"));
    toasts.info("Copied row");
  } catch {
    toasts.error("Couldn’t copy", "The clipboard isn’t available.");
  }
}
</script>

<template>
  <BaseDialog v-model:open="open" size="lg" :title="title">
    <dl
      v-if="cells"
      class="flex max-h-[60vh] flex-col divide-y divide-line overflow-auto"
    >
      <div
        v-for="(column, index) in columns"
        :key="index"
        class="grid grid-cols-[minmax(90px,180px)_1fr] gap-3 py-1.5"
      >
        <dt class="flex min-w-0 flex-col">
          <span class="truncate text-sm font-medium text-fg">{{
            column.name
          }}</span>
          <span class="truncate text-xs text-fg-subtle">
            {{ column.typeName.toLowerCase() }}
          </span>
        </dt>
        <dd class="min-w-0">
          <textarea
            v-if="editing === index"
            ref="box"
            v-model="draft"
            rows="2"
            class="w-full resize-y rounded-md border border-line bg-surface px-2 py-1 font-mono text-sm text-fg outline-2 -outline-offset-2 outline-accent"
            spellcheck="false"
            @keydown.enter.exact.prevent="commit()"
            @keydown.tab.prevent="commit(index + 1)"
            @keydown.esc.prevent="cancel"
            @blur="commit()"
          />
          <button
            v-else-if="editable"
            type="button"
            class="block w-full cursor-text rounded-md px-2 py-1 text-left font-mono text-sm break-words whitespace-pre-wrap hover:bg-surface-hover focus-visible:bg-surface-hover"
            @click="startEdit(index)"
          >
            <span
              v-if="shown(index).kind === 'null'"
              class="text-grid-null italic"
              >NULL</span
            >
            <span
              v-else-if="shown(index).kind === 'binary'"
              class="text-fg-muted"
              >{{ fullText(index) }}</span
            >
            <template v-else>{{ fullText(index) }}</template>
          </button>
          <div
            v-else
            class="block w-full rounded-md px-2 py-1 font-mono text-sm break-words whitespace-pre-wrap"
          >
            <span
              v-if="shown(index).kind === 'null'"
              class="text-grid-null italic"
              >NULL</span
            >
            <span
              v-else-if="shown(index).kind === 'binary'"
              class="text-fg-muted"
              >{{ fullText(index) }}</span
            >
            <template v-else>{{ fullText(index) }}</template>
          </div>
        </dd>
      </div>
    </dl>
    <p v-else class="py-6 text-center text-fg-subtle">This row has loaded.</p>
    <template #footer>
      <BaseButton class="mr-auto" :icon="Copy" @click="copyRow">
        Copy Row
      </BaseButton>
      <BaseButton variant="primary" @click="open = false">Done</BaseButton>
    </template>
  </BaseDialog>
</template>

<script setup>
import { computed, nextTick, ref, watch } from "vue";
import { displayText } from "../../lib/gridText.js";
import CellPicker from "./CellPicker.vue";

/** One grid row. With an `editor`, shows staged edits and the cell editor. */
const props = defineProps({
  index: { type: Number, required: true },
  cells: { type: Array, default: null },
  template: { type: String, required: true },
  numeric: { type: Array, required: true },
  selection: { type: Object, required: true },
  editor: { type: Object, default: null },
  /** Column being edited in this row (a display position), or -1. */
  editingCol: { type: Number, default: -1 },
  /** Display position to source column index; all columns when not given. */
  visible: { type: Array, default: null },
});
const emit = defineEmits(["celldown", "celldblclick", "commit", "cancel"]);

/** Display positions, which is what the template's columns decide. */
const columns = computed(
  () => props.visible ?? (props.cells ?? []).map((_, index) => index),
);

const input = ref(null);
const draft = ref("");
watch(
  () => props.editingCol,
  async (col) => {
    if (col < 0) return;
    draft.value = props.editor.valueOf(props.cells, source(col)) ?? "";
    await nextTick();
    input.value?.[0]?.focus();
    input.value?.[0]?.select();
  },
  { immediate: true },
);

const deleted = () => props.editor?.isDeleted(props.cells);
/** The source column a display position stands for. */
const source = (position) =>
  props.visible ? props.visible[position] : position;
/** What a display position shows: a staged edit when there is one. */
const shown = (position) => {
  const column = source(position);
  return props.editor
    ? props.editor.display(props.cells, column)
    : props.cells[column];
};

function onKeydown(event) {
  // Option/Alt+Return adds a line break; Return saves.
  if (event.key === "Enter" && !event.altKey) {
    event.preventDefault();
    emit("commit", draft.value, "down");
  } else if (event.key === "Tab") {
    event.preventDefault();
    emit("commit", draft.value, event.shiftKey ? "left" : "right");
  } else if (event.key === "Escape") {
    event.preventDefault();
    emit("cancel");
  }
  event.stopPropagation();
}

function onBlur() {
  if (props.editingCol >= 0) emit("commit", draft.value, null);
}
</script>

<template>
  <div
    role="row"
    :aria-rowindex="index + 2"
    class="absolute left-0 grid min-w-full"
    :class="[
      index % 2 && 'bg-grid-row-alt',
      cells && deleted() && 'bg-danger-subtle text-fg-muted line-through',
    ]"
    :style="{
      top: `${index * 26}px`,
      height: '26px',
      gridTemplateColumns: template,
    }"
  >
    <span
      class="sticky left-0 z-[1] border-r border-line bg-grid-header px-2 text-right leading-[26px] text-fg-disabled tabular-nums"
    >
      {{ (index + 1).toLocaleString() }}
    </span>
    <template v-if="cells">
      <template v-for="col in columns" :key="col">
        <span
          v-if="col === editingCol"
          role="gridcell"
          class="border-r border-line"
        >
          <CellPicker
            v-if="editor.pickerFor?.(source(col))"
            :picker="editor.pickerFor(source(col))"
            :initial="editor.valueOf(cells, source(col))"
            @commit="(value, move) => emit('commit', value, move)"
            @cancel="emit('cancel')"
          />
          <textarea
            v-else
            ref="input"
            v-model="draft"
            rows="1"
            aria-label="Edit value"
            class="block h-[26px] w-full resize-none overflow-hidden bg-surface px-2.5 leading-[26px] text-fg outline-2 -outline-offset-2 outline-accent"
            spellcheck="false"
            @keydown="onKeydown"
            @blur="onBlur"
          />
        </span>
        <span
          v-else
          role="gridcell"
          class="truncate border-r border-line px-2.5 leading-[26px]"
          :class="[
            numeric[col] &&
              shown(col).kind === 'value' &&
              'text-right tabular-nums',
            shown(col).kind === 'null' && 'text-grid-null italic',
            shown(col).kind === 'binary' && 'text-fg-muted',
            shown(col).kind === 'value' &&
              shown(col).value === '' &&
              'text-fg-subtle',
            editor?.isChanged(cells, source(col)) && 'bg-warning-subtle',
            selection.isSelected(index, col) && 'bg-selection',
            selection.focus.value.row === index &&
              selection.focus.value.col === col &&
              selection.active.value &&
              'outline-2 -outline-offset-2 outline-accent',
          ]"
          @mousedown="emit('celldown', $event, index, col)"
          @dblclick="emit('celldblclick', index, col)"
        >
          {{
            shown(col).kind === "value" && shown(col).value === ""
              ? "''"
              : displayText(shown(col))
          }}
        </span>
      </template>
    </template>
    <span v-else class="px-2.5 leading-[26px] text-fg-subtle">Loading…</span>
  </div>
</template>

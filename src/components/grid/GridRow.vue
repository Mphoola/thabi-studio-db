<script setup>
import { computed, nextTick, ref, watch } from "vue";
import { displayText } from "../../lib/gridText.js";
import { inTimeZone } from "../../lib/timeZones.js";
import { useAppSettings } from "../../composables/useAppSettings.js";
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

/** Display positions to source column index; all columns when not given. */
const columns = computed(
  () => props.visible ?? (props.cells ?? []).map((_, index) => index),
);
/** One cell per visible column, as positions. */
const positions = computed(() => columns.value.map((_, position) => position));

const input = ref(null);
const draft = ref("");
const deleted = () => props.editor?.isDeleted(props.cells);
/** The source column a display position stands for. */
const source = (position) =>
  props.visible ? props.visible[position] : position;

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

/** Shown for a column this row has no cell for. */
const EMPTY = Object.freeze({ kind: "null" });

/**
 * What a display position shows: a staged edit when there is one.
 *
 * Always a cell. A row can be asked about a column it does not carry — the
 * result's shape can change while the grid still holds a row from the old one —
 * and this runs during render, so returning nothing took the panel down.
 */
const shown = (position) => {
  const column = source(position);
  const cell = props.editor
    ? props.editor.display(props.cells, column)
    : props.cells?.[column];
  return cell ?? EMPTY;
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

const { settings } = useAppSettings();
/** What a cell shows: offsets converted to the chosen time zone. */
function label(cell) {
  const text = displayText(cell);
  return cell?.kind === "value"
    ? inTimeZone(text, settings.value.displayTimeZone)
    : text;
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
      <template v-for="position in positions" :key="position">
        <span
          v-if="position === editingCol"
          role="gridcell"
          class="border-r border-line"
        >
          <CellPicker
            v-if="editor.pickerFor?.(source(position))"
            :picker="editor.pickerFor(source(position))"
            :initial="editor.valueOf(cells, source(position))"
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
            numeric[position] &&
              shown(position).kind === 'value' &&
              'text-right tabular-nums',
            shown(position).kind === 'null' && 'text-grid-null italic',
            shown(position).kind === 'binary' && 'text-fg-muted',
            shown(position).kind === 'default' && 'text-grid-null italic',
            shown(position).kind === 'value' &&
              shown(position).value === '' &&
              'text-fg-subtle',
            editor?.isChanged(cells, source(position)) && 'bg-warning-subtle',
            selection.isSelected(index, position) && 'bg-selection',
            selection.focus.value.row === index &&
              selection.focus.value.position === position &&
              selection.active.value &&
              'outline-2 -outline-offset-2 outline-accent',
          ]"
          @mousedown="emit('celldown', $event, index, position)"
          @dblclick="emit('celldblclick', index, position)"
        >
          {{
            shown(position).kind === "value" && shown(position).value === ""
              ? "''"
              : label(shown(position))
          }}
        </span>
      </template>
    </template>
    <span v-else class="px-2.5 leading-[26px] text-fg-subtle">Loading…</span>
  </div>
</template>

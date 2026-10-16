<script setup>
import { computed, ref, watch } from "vue";
import { ChevronDown, ChevronUp, RotateCcw, Search } from "lucide-vue-next";
import { fuzzyFilter } from "../../lib/fuzzy.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import CheckboxInput from "../common/CheckboxInput.vue";
import TextInput from "../common/TextInput.vue";

/**
 * Choose which columns the grid shows and in what order. Search filters the
 * list; the arrows reorder one column at a time and work from the keyboard,
 * and dragging a row does the same thing with a pointer.
 */
const open = defineModel("open", { type: Boolean, default: false });
const props = defineProps({
  /** Every column, in source order. */
  columns: { type: Array, required: true },
  /** Source indexes to render, left to right. */
  visible: { type: Array, required: true },
  isHidden: { type: Function, required: true },
});
const emit = defineEmits(["toggle", "move", "reset", "fit"]);

const query = ref("");
/** The row being dragged within the list. */
const dragging = ref(null);
const dropOn = ref(null);
/** Off while a press starts on a control, so its click is not eaten. */
const draggable = ref(true);

function startPress(event) {
  draggable.value = !event.target.closest("input, button, label");
}

watch(open, (value) => {
  if (value) query.value = "";
});

/** Every column, in the order the grid shows them, then the hidden ones. */
const ordered = computed(() => {
  const byIndex = (index) =>
    props.columns[index] ?? { name: `Column ${index + 1}` };
  const shown = props.visible.map(byIndex);
  const rest = props.columns.filter(
    (column) => !props.visible.includes(props.columns.indexOf(column)),
  );
  return [...shown, ...rest];
});

const matches = computed(() => {
  const q = query.value.trim();
  return q
    ? fuzzyFilter(
        q,
        ordered.value,
        (column) => `${column.name} ${column.typeName}`,
      )
    : ordered.value;
});

function position(column) {
  return ordered.value.indexOf(column);
}

function dropOnColumn(column) {
  const from = dragging.value;
  dropOn.value = null;
  dragging.value = null;
  if (!from || from === column) return;
  const delta = position(column) - position(from);
  if (delta !== 0) emit("move", from.name, delta);
}
</script>

<template>
  <BaseDialog v-model:open="open" size="sm" title="Columns">
    <div class="flex flex-col gap-2">
      <TextInput
        v-model="query"
        :icon="Search"
        placeholder="Search columns"
        aria-label="Search columns"
      />
      <ul
        class="max-h-72 overflow-auto rounded-md border border-line"
        role="list"
      >
        <li
          v-for="column in matches"
          :key="column.name"
          :draggable="draggable"
          class="flex items-center gap-2 border-b border-line px-2 py-1 last:border-b-0"
          :class="dropOn === column ? 'bg-accent-subtle' : ''"
          @mousedown="startPress"
          @dragstart="
            dragging = column;
            draggable = true;
          "
          @dragover.prevent="dropOn = column"
          @dragleave="dropOn === column && (dropOn = null)"
          @drop="dropOnColumn(column)"
          @dragend="
            dragging = null;
            dropOn = null;
          "
        >
          <CheckboxInput
            :model-value="!isHidden(column.name)"
            :label="column.name"
            :aria-label="`Show ${column.name}`"
            @update:model-value="emit('toggle', column.name)"
          />
          <span class="min-w-0 flex-1 truncate text-xs text-fg-subtle">
            {{ column.typeName }}
          </span>
          <BaseButton
            size="sm"
            variant="ghost"
            :icon="ChevronUp"
            :label="`Move ${column.name} earlier`"
            :disabled="position(column) === 0"
            @click="emit('move', column.name, -1)"
          />
          <BaseButton
            size="sm"
            variant="ghost"
            :icon="ChevronDown"
            :label="`Move ${column.name} later`"
            :disabled="position(column) === ordered.length - 1"
            @click="emit('move', column.name, 1)"
          />
        </li>
        <li v-if="!matches.length" class="px-3 py-4 text-center text-fg-subtle">
          No columns match “{{ query }}”.
        </li>
      </ul>
    </div>
    <template #footer>
      <BaseButton
        class="mr-auto"
        size="sm"
        :icon="RotateCcw"
        @click="emit('reset')"
      >
        Reset
      </BaseButton>
      <BaseButton size="sm" @click="emit('fit')">Fit to Contents</BaseButton>
      <BaseButton size="sm" variant="primary" @click="open = false"
        >Done</BaseButton
      >
    </template>
  </BaseDialog>
</template>

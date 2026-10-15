<script setup>
import { ArrowDown, ArrowUp } from "lucide-vue-next";

/** Sticky column headers: name, type, sort indicator, resize handle. */
defineProps({
  columns: { type: Array, required: true },
  template: { type: String, required: true },
  widths: { type: Array, required: true },
  /** Display position to source column index, for the copy menu later. */
  visible: { type: Array, default: null },
  sort: { type: Object, default: null },
  sortable: { type: Boolean, default: true },
});
const emit = defineEmits(["sort", "resize"]);

function startResize(event, index, startWidth) {
  event.preventDefault();
  event.stopPropagation();
  const handle = event.currentTarget;
  handle.setPointerCapture(event.pointerId);
  const x = event.clientX;
  const move = (e) => emit("resize", index, startWidth + e.clientX - x);
  const up = () => {
    handle.removeEventListener("pointermove", move);
    handle.removeEventListener("pointerup", up);
  };
  handle.addEventListener("pointermove", move);
  handle.addEventListener("pointerup", up);
}
</script>

<template>
  <div
    role="row"
    aria-rowindex="1"
    class="sticky top-0 z-[2] grid min-w-full border-b border-line bg-grid-header font-sans"
    :style="{ gridTemplateColumns: template, height: '26px' }"
  >
    <span class="sticky left-0 z-[3] border-r border-line bg-grid-header" />
    <div
      v-for="(column, index) in columns"
      :key="index"
      role="columnheader"
      :aria-sort="
        sort?.column === index
          ? sort.descending
            ? 'descending'
            : 'ascending'
          : 'none'
      "
      class="group relative flex min-w-0 items-center gap-1 border-r border-line px-2.5"
    >
      <button
        type="button"
        class="flex min-w-0 flex-1 items-center gap-1.5 text-left text-xs font-semibold text-fg-muted disabled:cursor-default"
        :disabled="!sortable"
        :title="`${column.name} · ${column.typeName.toLowerCase()}`"
        @click="emit('sort', index)"
      >
        <span class="truncate">{{ column.name }}</span>
        <span class="truncate font-normal text-fg-subtle">
          {{ column.typeName.toLowerCase() }}
        </span>
        <component
          :is="sort.descending ? ArrowDown : ArrowUp"
          v-if="sort?.column === index"
          :size="11"
          :stroke-width="2"
          class="shrink-0 text-accent-text"
          aria-hidden="true"
        />
      </button>
      <span
        class="absolute inset-y-0 -right-1 z-[1] w-2 cursor-col-resize hover:bg-accent-subtle"
        aria-hidden="true"
        @pointerdown="startResize($event, index, widths[index])"
        @click.stop
      />
    </div>
  </div>
</template>

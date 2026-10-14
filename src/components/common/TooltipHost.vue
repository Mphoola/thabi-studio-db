<script setup>
import { nextTick, ref, watch } from "vue";
import { useTooltipState } from "../../composables/useTooltip.js";
import { placeFloating } from "../../lib/position.js";
import KbdShortcut from "./KbdShortcut.vue";

const { state } = useTooltipState();
const el = ref(null);
const position = ref({ top: 0, left: 0 });

// popover="manual" puts the tooltip in the top layer, above open dialogs.
watch(state, async (value) => {
  const node = el.value;
  if (!node) return;
  if (!value) {
    if (node.matches(":popover-open")) node.hidePopover();
    return;
  }
  if (!node.matches(":popover-open")) node.showPopover();
  await nextTick();
  const rect = node.getBoundingClientRect();
  position.value = placeFloating(value.anchor, rect, {
    align: "center",
    gap: 6,
  });
});
</script>

<template>
  <div
    ref="el"
    popover="manual"
    role="tooltip"
    class="pointer-events-none fixed m-0 flex items-center gap-2 rounded-md border border-line bg-surface-raised px-2 py-1 text-sm text-fg shadow-popover"
    :style="{
      inset: 'auto',
      top: `${position.top}px`,
      left: `${position.left}px`,
    }"
  >
    <template v-if="state">
      <span>{{ state.text }}</span>
      <KbdShortcut v-if="state.shortcut" :shortcut="state.shortcut" />
    </template>
  </div>
</template>

<script setup>
/**
 * The diagram's header: what it is showing, and the controls for it.
 *
 * Its own component so DiagramView can be about the data and the drawing
 * rather than about button order.
 */
import { Download, Minus, Network, Plus, RefreshCw } from "lucide-vue-next";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";

defineProps({
  title: { type: String, required: true },
  tables: { type: Number, required: true },
  keys: { type: Number, required: true },
  zoomPercent: { type: Number, required: true },
  loading: { type: Boolean, default: false },
  canExport: { type: Boolean, default: false },
});
const emit = defineEmits(["zoom-out", "zoom-in", "fit", "export", "reload"]);
</script>

<template>
  <header
    class="flex shrink-0 items-center gap-1.5 border-b border-line px-3 py-1.5"
  >
    <Network
      :size="16"
      :stroke-width="1.75"
      class="shrink-0 text-fg-muted"
      aria-hidden="true"
    />
    <span class="shrink-0 truncate text-sm font-medium">{{ title }}</span>
    <span class="shrink-0 text-xs text-fg-subtle">
      {{ tables }} tables · {{ keys }} keys
    </span>
    <span
      class="shrink-0 rounded border border-line px-1 py-px text-[10px] tracking-wide text-fg-subtle uppercase"
    >
      Read-only
    </span>

    <div class="ml-auto flex shrink-0 items-center gap-0.5">
      <span class="mr-1 w-9 text-right text-xs text-fg-subtle tabular-nums">
        {{ zoomPercent }}%
      </span>
      <IconButton :icon="Minus" label="Zoom out" @click="emit('zoom-out')" />
      <BaseButton size="sm" variant="ghost" @click="emit('fit')"
        >Fit</BaseButton
      >
      <IconButton :icon="Plus" label="Zoom in" @click="emit('zoom-in')" />
      <span class="mx-1 h-4 w-px bg-line" />
      <IconButton
        :icon="Download"
        label="Export SVG"
        :disabled="!canExport"
        @click="emit('export')"
      />
      <IconButton
        :icon="RefreshCw"
        label="Reload"
        :disabled="loading"
        @click="emit('reload')"
      />
    </div>
  </header>
</template>

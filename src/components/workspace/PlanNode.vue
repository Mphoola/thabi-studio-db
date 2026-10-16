<script setup>
import { computed } from "vue";
import { Flame } from "lucide-vue-next";

/** One step of a query plan, with its children nested below. */
const props = defineProps({
  node: { type: Object, required: true },
  total: { type: Number, default: 0 },
  own: { type: Function, required: true },
  hottest: { type: Set, required: true },
  depth: { type: Number, default: 0 },
});
const share = computed(() =>
  props.total ? Math.min(100, (props.own(props.node) / props.total) * 100) : 0,
);
const hot = computed(() => props.hottest.has(props.node));
const number = (value) =>
  value === null || value === undefined
    ? "—"
    : value.toLocaleString(undefined, { maximumFractionDigits: 2 });
</script>

<template>
  <div role="treeitem" :aria-expanded="node.children.length ? true : undefined">
    <div
      class="flex items-start gap-3 rounded-md px-2 py-1 hover:bg-surface-hover"
      :style="{ paddingLeft: `${depth * 18 + 8}px` }"
    >
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-1.5 text-sm text-fg">
          <Flame
            v-if="hot"
            :size="14"
            :stroke-width="1.75"
            class="shrink-0 text-warning-text"
            aria-label="One of the most expensive steps"
          />
          <span class="truncate font-medium">{{ node.label }}</span>
        </div>
        <div
          v-if="node.detail"
          class="truncate font-mono text-xs text-fg-subtle"
          :title="node.detail"
        >
          {{ node.detail }}
        </div>
      </div>
      <div class="w-28 shrink-0 pt-1" aria-hidden="true">
        <div class="h-1.5 rounded-full bg-surface-active">
          <div
            class="h-1.5 rounded-full"
            :class="hot ? 'bg-warning' : 'bg-accent'"
            :style="{ width: `${share}%` }"
          />
        </div>
      </div>
      <div
        class="w-36 shrink-0 text-right font-mono text-xs text-fg-muted tabular-nums"
      >
        cost {{ number(node.cost) }} · rows {{ number(node.rows) }}
      </div>
    </div>
    <PlanNode
      v-for="(child, index) in node.children"
      :key="index"
      :node="child"
      :total="total"
      :own="own"
      :hottest="hottest"
      :depth="depth + 1"
    />
  </div>
</template>

<script setup>
import { computed, ref } from "vue";
import SegmentedControl from "../common/SegmentedControl.vue";
import PlanNode from "./PlanNode.vue";

/**
 * A query plan as a tree: each step with its cost and rows, a bar for its
 * share of the total cost, and the most expensive steps marked.
 */
const props = defineProps({ plan: { type: Object, required: true } });
const mode = ref("tree");
const modes = [
  { value: "tree", label: "Plan" },
  { value: "raw", label: "Raw" },
];

function walk(nodes, out = []) {
  for (const node of nodes) {
    out.push(node);
    walk(node.children, out);
  }
  return out;
}
const all = computed(() => walk(props.plan.nodes));
const total = computed(() =>
  Math.max(0, ...props.plan.nodes.map((n) => n.cost ?? 0)),
);
/** A node's own cost: its total minus its children's. */
const own = (node) =>
  Math.max(
    0,
    (node.cost ?? 0) - node.children.reduce((sum, c) => sum + (c.cost ?? 0), 0),
  );
/** The three steps that cost the most on their own. */
const hottest = computed(
  () =>
    new Set(
      [...all.value]
        .filter((n) => own(n) > 0)
        .sort((a, b) => own(b) - own(a))
        .slice(0, 3),
    ),
);
</script>

<template>
  <div class="flex min-h-0 flex-col">
    <div
      class="flex h-9 shrink-0 items-center gap-3 border-b border-line px-3 text-xs text-fg-muted"
    >
      <SegmentedControl v-model="mode" :options="modes" label="Plan view" />
      <span>
        {{
          plan.analyzed
            ? "Ran with ANALYZE, then rolled back"
            : "Estimated plan"
        }}
        <template v-if="total">
          · total cost {{ total.toLocaleString() }}</template
        >
      </span>
    </div>
    <div
      v-if="mode === 'tree'"
      class="min-h-0 flex-1 overflow-auto p-3"
      role="tree"
      aria-label="Query plan"
    >
      <PlanNode
        v-for="(node, index) in plan.nodes"
        :key="index"
        :node="node"
        :total="total"
        :own="own"
        :hottest="hottest"
      />
    </div>
    <pre
      v-else
      class="selectable min-h-0 flex-1 overflow-auto p-3 font-mono text-sm whitespace-pre-wrap text-fg"
      >{{ plan.raw }}</pre>
  </div>
</template>

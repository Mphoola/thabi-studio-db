<script setup>
import { computed } from "vue";
import { useExplorer } from "../../composables/useExplorer.js";
import { useExplorerActions } from "./useExplorerActions.js";

/** Inline message under an expanded node: empty, error or "switch database". */
const props = defineProps({
  node: { type: Object, required: true },
  level: { type: Number, required: true },
});
const explorer = useExplorer();
const actions = useExplorerActions();
const state = computed(() => explorer.stateOf(props.node.key));
const error = computed(() => state.value.error);
const empty = computed(
  () => state.value.status === "loaded" && !state.value.children?.length,
);
const emptyText = computed(() =>
  props.node.type === "database"
    ? `Nothing to show in “${props.node.label}”.`
    : `No tables or views in “${props.node.label}”.`,
);
</script>

<template>
  <li
    v-if="empty || state.status === 'error'"
    role="none"
    class="flex flex-col gap-1 py-1 pr-2 text-xs"
    :style="{ paddingLeft: `${(level - 1) * 14 + 24}px` }"
  >
    <span v-if="empty" class="text-fg-subtle">{{ emptyText }}</span>
    <template v-else>
      <span
        :class="
          error?.kind === 'database_not_open'
            ? 'text-fg-muted'
            : 'text-danger-text'
        "
      >
        {{ error?.message }}
      </span>
      <button
        v-if="error?.kind === 'database_not_open'"
        type="button"
        class="self-start rounded-sm text-accent-text hover:underline"
        @click="actions.run('switch', node)"
      >
        Switch Database
      </button>
      <button
        v-else
        type="button"
        class="self-start rounded-sm text-accent-text hover:underline"
        @click="explorer.refresh(node)"
      >
        Try Again
      </button>
    </template>
  </li>
</template>

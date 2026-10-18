<script setup>
/**
 * What the diagram shows instead of a drawing: a failure, a wait, or nothing
 * to draw — and for the last of those, why.
 *
 * Its own component because "no tables" is a message with a cause behind it,
 * and the cause is worked out by asking the server. That reasoning is a page's
 * worth of prose in a template and does not belong in the middle of one.
 */
import { Network, Table2 } from "lucide-vue-next";
import EmptyState from "../common/EmptyState.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";

defineProps({
  failed: { type: String, default: "" },
  loading: { type: Boolean, default: false },
  emptyReason: { type: String, default: "" },
  arranging: { type: Boolean, default: false },
  hasTables: { type: Boolean, default: false },
  database: { type: String, default: "" },
  schema: { type: String, default: "" },
});
</script>

<template>
  <EmptyState v-if="failed" :icon="Network" title="Couldn’t draw the diagram">
    <!-- The server's own reason, not just "it failed". Repeating the message
         without the detail has cost more time than it saved. -->
    <template #message>{{ failed }}</template>
    <p class="mt-2 text-xs text-fg-subtle">
      {{ database }}{{ schema ? `.${schema}` : "" }}
    </p>
  </EmptyState>

  <EmptyState
    v-else-if="loading && !hasTables"
    :icon="Network"
    title="Reading the schema…"
  />

  <EmptyState v-else-if="!hasTables" :icon="Table2" title="Nothing to draw">
    <template #message>
      {{ emptyReason || "No tables came back for this database." }}
    </template>
  </EmptyState>

  <div
    v-else-if="arranging"
    class="pointer-events-none absolute bottom-2 left-3 flex items-center gap-2 text-xs text-fg-subtle"
  >
    <SpinnerIcon />
    Arranging…
  </div>
</template>

<script setup>
/**
 * What the dialog shows while a dump runs.
 *
 * Its own component because it replaces the form rather than sitting beside
 * it: while a table of fifty thousand rows is being written, the choices that
 * produced it are not what anyone is looking at, and a Stop button on its own
 * with no indication of movement reads as a hang.
 */
import { Loader2 } from "lucide-vue-next";
import BaseButton from "../common/BaseButton.vue";

defineProps({
  /** { table, tables, rows } from Rust, or null before the first report. */
  progress: { type: Object, default: null },
  where: { type: String, default: "" },
});
defineEmits(["stop"]);
</script>

<template>
  <section class="space-y-3 py-6 text-center">
    <Loader2
      :size="22"
      :stroke-width="1.75"
      class="mx-auto animate-spin text-fg-muted"
      aria-hidden="true"
    />
    <p class="text-sm">
      <template v-if="progress">
        Table {{ progress.tables }} —
        <span class="font-mono">{{ progress.table }}</span>
      </template>
      <template v-else>Working out what to write…</template>
    </p>
    <p v-if="progress" class="text-xs text-fg-subtle tabular-nums">
      {{ progress.rows.toLocaleString() }} rows from this table
    </p>
    <p class="text-xs text-fg-subtle">{{ where }}</p>
    <BaseButton size="sm" @click="$emit('stop')">Stop</BaseButton>
  </section>
</template>

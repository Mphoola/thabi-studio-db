<script setup>
import { computed } from "vue";
import { Database } from "lucide-vue-next";

/** The database engine, in its engine colour. */
const props = defineProps({
  driver: { type: String, required: true },
  compact: { type: Boolean, default: false },
});
const engines = {
  postgres: { label: "PostgreSQL", short: "PG", color: "text-engine-postgres" },
  mysql: { label: "MySQL", short: "My", color: "text-engine-mysql" },
  sqlserver: {
    label: "SQL Server",
    short: "MS",
    color: "text-engine-sqlserver",
  },
};
const engine = computed(
  () =>
    engines[props.driver] ?? {
      label: props.driver,
      short: "?",
      color: "text-fg-subtle",
    },
);
</script>

<template>
  <span
    class="inline-flex shrink-0 items-center gap-1 text-xs font-medium"
    :class="engine.color"
    :title="engine.label"
  >
    <Database :size="13" :stroke-width="1.75" aria-hidden="true" />
    <span v-if="!compact">{{ engine.label }}</span>
    <span v-else class="sr-only">{{ engine.label }}</span>
  </span>
</template>

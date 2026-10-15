<script setup>
import { computed } from "vue";
import { Lock, ShieldAlert, ShieldCheck } from "lucide-vue-next";
import { vTooltip } from "../../composables/useTooltip.js";
import { safeModeFor } from "../../lib/sqlSafety.js";

/**
 * Shows a connection's safe mode: a shield for the modes that ask, a lock for
 * read-only, nothing for "off". Used wherever connections are listed, so the
 * mode is visible without opening the connection.
 */
const props = defineProps({
  /** "off", "confirm-writes", "confirm-everything" or "read-only". */
  safeMode: { type: String, default: "" },
  /** Also show the mode's name, for places with room. */
  withLabel: { type: Boolean, default: false },
});

const ICONS = {
  "confirm-writes": ShieldCheck,
  "confirm-everything": ShieldAlert,
  "read-only": Lock,
};

const mode = computed(() => safeModeFor(props.safeMode));
const icon = computed(() => ICONS[mode.value.value] ?? null);
// "Off" is the absence of a guard, so it gets no badge.
const shown = computed(() => Boolean(icon.value) && props.safeMode !== "off");
const tooltip = computed(() => `${mode.value.label} — ${mode.value.hint}`);
</script>

<template>
  <span
    v-if="shown"
    v-tooltip="tooltip"
    class="flex shrink-0 items-center gap-1 text-fg-subtle"
    :class="safeMode === 'read-only' && 'text-warning-text'"
  >
    <component :is="icon" :size="14" :stroke-width="1.75" aria-hidden="true" />
    <span v-if="withLabel" class="text-xs">{{ mode.label }}</span>
    <span class="sr-only">{{ mode.label }}</span>
  </span>
</template>

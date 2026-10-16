<script setup>
import { computed } from "vue";
import { vTooltip } from "../../composables/useTooltip.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { environmentFor } from "../../lib/environments.js";
import { connectionSummary } from "../../lib/connectionSummary.js";
import { safeModeFor } from "../../lib/sqlSafety.js";
import { runCommand } from "../../composables/useCommands.js";
import SafeModeBadge from "../connections/SafeModeBadge.vue";

// The pill takes the connection's tag colour, so Production looks different
// from Local at a glance. Literal classes, so Tailwind generates them.
const TINTS = {
  red: "border-tag-red/40 bg-tag-red/10",
  orange: "border-tag-orange/40 bg-tag-orange/10",
  blue: "border-tag-blue/40 bg-tag-blue/10",
  teal: "border-tag-teal/40 bg-tag-teal/10",
  gray: "border-line bg-surface",
};
const LABELS = {
  red: "text-tag-red",
  orange: "text-tag-orange",
  blue: "text-tag-blue",
  teal: "text-tag-teal",
  gray: "text-fg-muted",
};

const store = useConnectionProfiles();
const { selectedProfile } = store;
const environment = computed(() =>
  selectedProfile.value
    ? environmentFor(selectedProfile.value.environment)
    : null,
);
const status = computed(() =>
  selectedProfile.value ? store.statusFor(selectedProfile.value.id) : null,
);
const connected = computed(() => status.value?.state === "connected");
const connecting = computed(() => status.value?.state === "connecting");
const parts = computed(() =>
  selectedProfile.value
    ? connectionSummary(selectedProfile.value, status.value)
    : [],
);
const safeMode = computed(() =>
  selectedProfile.value ? safeModeFor(selectedProfile.value.safeMode) : null,
);
const guarded = computed(
  () => selectedProfile.value && selectedProfile.value.safeMode !== "off",
);

const tooltip = computed(() => {
  if (!selectedProfile.value) return "No connection selected";
  const state = connected.value
    ? "Connected"
    : connecting.value
      ? "Connecting…"
      : "Not connected";
  const mode = `Safe mode: ${safeMode.value.label}`;
  return `${state} · ${parts.value.map((part) => part.text).join(" · ")} · ${mode}`;
});

const TONES = {
  strong: "font-medium text-fg",
  warning: "text-warning-text",
};
</script>

<template>
  <button
    v-tooltip="tooltip"
    type="button"
    class="flex h-6 max-w-[min(46rem,48vw)] min-w-0 items-center gap-2 rounded-md border px-2.5 text-sm text-fg-muted transition-colors hover:border-line-strong"
    :class="TINTS[environment?.token ?? 'gray']"
    :aria-label="tooltip"
    @click="runCommand('view.connections')"
  >
    <template v-if="selectedProfile">
      <span
        class="size-2 shrink-0 rounded-full"
        :class="
          connected
            ? 'bg-success'
            : connecting
              ? 'animate-pulse bg-warning'
              : 'bg-tag-gray'
        "
        aria-hidden="true"
      />
      <span
        class="shrink-0 text-xs font-semibold tracking-wide uppercase"
        :class="LABELS[environment.token]"
      >
        {{ environment.label }}
      </span>
      <span class="h-3.5 w-px shrink-0 bg-line-strong" aria-hidden="true" />
      <span class="block min-w-0 truncate" :class="!connected && 'opacity-70'">
        <template v-for="(part, index) in parts" :key="part.key">
          <span v-if="index" class="px-1 text-fg-subtle" aria-hidden="true"
            >:</span
          >
          <span :class="TONES[part.tone]">{{ part.text }}</span>
        </template>
      </span>
      <template v-if="guarded">
        <span class="h-3.5 w-px shrink-0 bg-line-strong" aria-hidden="true" />
        <SafeModeBadge :safe-mode="selectedProfile.safeMode" with-label />
      </template>
    </template>
    <span v-else>No connection</span>
  </button>
</template>

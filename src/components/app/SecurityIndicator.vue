<script setup>
import { computed } from "vue";
import { Lock, LockOpen, ShieldAlert, Waypoints } from "lucide-vue-next";
import { vTooltip } from "../../composables/useTooltip.js";
import { isLoopbackHost } from "../../lib/tls.js";

/** How a connection is protected, from its saved settings. */
const props = defineProps({ profile: { type: Object, required: true } });

const tls = computed(() => {
  const { tlsMode, host, sshEnabled } = props.profile;
  if (tlsMode === "verify-full")
    return { icon: Lock, label: "Verified TLS", tone: "text-fg-muted" };
  if (tlsMode === "require")
    return {
      icon: ShieldAlert,
      label: "Encrypted, unverified",
      tone: "text-warning-text",
    };
  const safe = isLoopbackHost(host) || sshEnabled;
  return {
    icon: LockOpen,
    label: "Not encrypted",
    tone: safe ? "text-fg-muted" : "text-warning-text",
  };
});

const tooltip = computed(() => {
  const parts = [`TLS: ${tls.value.label}`];
  if (props.profile.sshEnabled)
    parts.push(`SSH tunnel through ${props.profile.sshHost}`);
  return parts.join(" · ");
});
</script>

<template>
  <span
    v-tooltip="tooltip"
    class="flex items-center gap-1"
    :class="tls.tone"
    tabindex="0"
    :aria-label="tooltip"
  >
    <component
      :is="tls.icon"
      :size="12"
      :stroke-width="1.75"
      aria-hidden="true"
    />
    {{ tls.label }}
    <template v-if="profile.sshEnabled">
      <Waypoints
        :size="12"
        :stroke-width="1.75"
        class="ml-1.5"
        aria-hidden="true"
      />
      via SSH
    </template>
  </span>
</template>

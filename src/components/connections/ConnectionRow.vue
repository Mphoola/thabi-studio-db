<script setup>
import { computed } from "vue";
import {
  Copy,
  Lock,
  LockOpen,
  Pencil,
  ShieldAlert,
  Trash2,
  Waypoints,
} from "lucide-vue-next";
import { useConnectionActions } from "../../composables/useConnectionActions.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { vTooltip } from "../../composables/useTooltip.js";
import { relativeTime } from "../../lib/relativeTime.js";
import { isLoopbackHost } from "../../lib/tls.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";
import EngineBadge from "./EngineBadge.vue";

/** One saved connection. Double-click or Return connects. */
const props = defineProps({ profile: { type: Object, required: true } });
const store = useConnectionProfiles();
const actions = useConnectionActions();

const status = computed(() => store.statusFor(props.profile.id));
const connected = computed(() => status.value?.state === "connected");
const selected = computed(
  () => store.selectedProfile.value?.id === props.profile.id,
);
const tls = computed(() => {
  const { tlsMode, host, sshEnabled } = props.profile;
  if (tlsMode === "verify-full")
    return { icon: Lock, label: "Verified TLS", tone: "text-fg-subtle" };
  if (tlsMode === "require")
    return {
      icon: ShieldAlert,
      label: "Encrypted, not verified",
      tone: "text-warning-text",
    };
  const safe = isLoopbackHost(host) || sshEnabled;
  return {
    icon: LockOpen,
    label: "Not encrypted",
    tone: safe ? "text-fg-subtle" : "text-warning-text",
  };
});
const lastUsed = computed(() => relativeTime(props.profile.lastConnectedAt));

function onKeydown(event) {
  if (event.target !== event.currentTarget) return;
  if (event.key === "Enter") {
    event.preventDefault();
    actions.toggle(props.profile);
  } else if (event.key === "Backspace" && event.metaKey) {
    event.preventDefault();
    actions.remove(props.profile);
  }
}
</script>

<template>
  <li
    tabindex="0"
    class="group flex min-w-0 cursor-default items-center gap-3 px-3 py-2.5 outline-none hover:bg-surface-hover focus-visible:bg-surface-hover"
    :class="selected && 'bg-accent-subtle hover:bg-accent-subtle'"
    :aria-label="`${profile.name}, ${connected ? 'connected' : 'not connected'}`"
    @dblclick="actions.toggle(profile)"
    @keydown="onKeydown"
  >
    <div class="flex min-w-0 flex-1 flex-col gap-0.5">
      <div class="flex min-w-0 items-center gap-2">
        <span class="truncate font-medium text-fg">{{ profile.name }}</span>
        <EngineBadge :driver="profile.driver" />
        <span v-if="connected" class="truncate text-xs text-success-text">
          Connected · {{ status.server?.version }}
        </span>
        <span
          v-else-if="status?.state === 'connecting'"
          class="text-xs text-fg-subtle"
        >
          Connecting…
        </span>
      </div>
      <div class="flex min-w-0 items-center gap-2 text-sm text-fg-muted">
        <span class="truncate font-mono">
          {{ profile.username }}@{{ profile.host }}:{{ profile.port
          }}<template v-if="profile.databaseName"
            >/{{ profile.databaseName }}</template
          >
        </span>
        <span
          v-tooltip="tls.label"
          class="shrink-0"
          :class="tls.tone"
          :aria-label="tls.label"
        >
          <component :is="tls.icon" :size="12" :stroke-width="1.75" />
        </span>
        <span
          v-if="profile.sshEnabled"
          v-tooltip="`SSH through ${profile.sshHost}`"
          class="flex shrink-0 items-center gap-1 text-xs text-fg-subtle"
        >
          <Waypoints :size="12" :stroke-width="1.75" aria-hidden="true" /> SSH
        </span>
        <span v-if="lastUsed" class="ml-auto shrink-0 text-xs text-fg-subtle">
          {{ lastUsed }}
        </span>
      </div>
    </div>
    <BaseButton
      size="sm"
      :variant="connected ? 'secondary' : 'primary'"
      :loading="actions.busy.value.has(profile.id)"
      @click="actions.toggle(profile)"
    >
      {{ connected ? "Disconnect" : "Connect" }}
    </BaseButton>
    <div
      class="flex opacity-60 group-focus-within:opacity-100 group-hover:opacity-100"
    >
      <IconButton
        :icon="Pencil"
        size="sm"
        label="Edit Connection"
        @click="actions.edit(profile)"
      />
      <IconButton
        :icon="Copy"
        size="sm"
        label="Duplicate Connection"
        @click="actions.duplicate(profile)"
      />
      <IconButton
        :icon="Trash2"
        size="sm"
        label="Delete Connection"
        @click="actions.remove(profile)"
      />
    </div>
  </li>
</template>

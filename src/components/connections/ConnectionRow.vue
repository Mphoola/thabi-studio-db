<script setup>
import { isModKey } from "../../lib/platform.js";
import { computed } from "vue";
import {
  Clock,
  Copy,
  GripVertical,
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
import { colourDotClass } from "../../lib/connectionGroups.js";
import { describeLimit, overridesDefaults } from "../../lib/limits.js";
import { relativeTime } from "../../lib/relativeTime.js";
import { isLoopbackHost } from "../../lib/tls.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";
import EngineBadge from "./EngineBadge.vue";
import SafeModeBadge from "./SafeModeBadge.vue";

/** One saved connection. Double-click or Return connects. */
const props = defineProps({
  profile: { type: Object, required: true },
  /** Highlights the row while it is being dragged or moved. */
  moving: { type: Boolean, default: false },
});
const emit = defineEmits(["move"]);
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
/** Only a custom colour gets its own dot; otherwise the group carries it. */
const customColour = computed(() => props.profile.colour || "");
/** This connection's own timeout or row limit, when it overrides the app's. */
const limits = computed(() => {
  if (!overridesDefaults(props.profile)) return null;
  const parts = [];
  if (props.profile.queryTimeoutSeconds)
    parts.push(describeLimit(props.profile.queryTimeoutSeconds, "seconds"));
  if (props.profile.maxResultRows)
    parts.push(describeLimit(props.profile.maxResultRows, "rows"));
  return parts.join(" · ");
});

function onKeydown(event) {
  if (event.target !== event.currentTarget) return;
  if (event.key === "Enter") {
    event.preventDefault();
    actions.toggle(props.profile);
  } else if (
    event.altKey &&
    (event.key === "ArrowUp" || event.key === "ArrowDown")
  ) {
    // Alt+Up / Alt+Down move the connection within its group.
    event.preventDefault();
    emit("move", event.key === "ArrowUp" ? -1 : 1);
  } else if (event.key === "Backspace" && isModKey(event)) {
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
    :aria-label="`${profile.name}, ${connected ? 'connected' : 'not connected'}${customColour ? `, ${customColour}` : ''}`"
    @dblclick="actions.toggle(profile)"
    @keydown="onKeydown"
  >
    <GripVertical
      :size="14"
      :stroke-width="1.75"
      class="-ml-1 shrink-0 text-fg-subtle opacity-0 group-hover:opacity-60 focus-visible:opacity-60"
      aria-hidden="true"
    />
    <div class="flex min-w-0 flex-1 flex-col gap-0.5">
      <div class="flex min-w-0 items-center gap-2">
        <span
          v-if="customColour"
          v-tooltip="`${customColour} connection`"
          class="size-2 shrink-0 rounded-full"
          :class="colourDotClass(profile)"
          :aria-label="`${customColour} connection`"
        />
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
        <span
          v-if="limits"
          v-tooltip="`Own limits: ${limits}. Settings show the defaults.`"
          class="flex shrink-0 items-center gap-1 text-xs text-fg-subtle"
        >
          <Clock :size="12" :stroke-width="1.75" aria-hidden="true" />
          {{ limits }}
          <span class="sr-only">Own query limits: {{ limits }}</span>
        </span>
        <SafeModeBadge
          :safe-mode="profile.safeMode"
          class="text-xs"
          with-label
        />
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

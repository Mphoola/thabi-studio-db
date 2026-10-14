<script setup>
import { ref } from "vue";
import { Copy, Pencil, Trash2 } from "lucide-vue-next";
import { confirm } from "../../composables/useConfirm.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail, toUserMessage } from "../../lib/errors.js";
import { environmentFor } from "../../lib/environments.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";

const store = useConnectionProfiles();
const toasts = useToasts();
const busyProfileIds = ref(new Set());

function markBusy(id, busy) {
  const next = new Set(busyProfileIds.value);
  if (busy) next.add(id);
  else next.delete(id);
  busyProfileIds.value = next;
}

async function duplicate(profile) {
  try {
    const warning = await store.duplicate(profile.id);
    if (warning)
      toasts.error("Duplicated, but Keychain needs attention", warning);
  } catch (error) {
    toasts.error(`Couldn’t duplicate “${profile.name}”`, toUserMessage(error));
  }
}

async function remove(profile) {
  const confirmed = await confirm({
    title: `Delete “${profile.name}”?`,
    message:
      "Its saved password is also removed from your keychain. This can't be undone.",
    confirmLabel: "Delete",
    destructive: true,
  });
  if (!confirmed) return;

  try {
    await store.remove(profile.id);
  } catch (error) {
    toasts.error(`Couldn’t delete “${profile.name}”`, toUserMessage(error));
  }
}

async function select(profile) {
  try {
    await store.select(profile.id);
  } catch (error) {
    toasts.error(`Couldn’t select “${profile.name}”`, toUserMessage(error));
  }
}

async function toggleConnection(profile) {
  const wasConnected = store.isConnected(profile.id);
  markBusy(profile.id, true);
  try {
    if (wasConnected) {
      await store.disconnect(profile.id);
    } else {
      await store.select(profile.id);
      await store.connect(profile.id);
    }
  } catch (error) {
    toasts.error(
      wasConnected
        ? `Couldn’t disconnect from “${profile.name}”`
        : `Couldn’t connect to “${profile.name}”`,
      toUserDetail(error),
    );
  } finally {
    markBusy(profile.id, false);
  }
}
</script>

<template>
  <ul class="flex flex-col divide-y divide-line rounded-lg border border-line">
    <li
      v-for="profile in store.profiles.value"
      :key="profile.id"
      class="flex min-w-0 items-center gap-3 px-3 py-2.5"
    >
      <span
        class="size-2 shrink-0 rounded-full"
        :class="environmentFor(profile.environment).dotClass"
        aria-hidden="true"
      />
      <div class="flex min-w-0 flex-1 flex-col gap-1">
        <div class="flex min-w-0 items-center gap-2">
          <span class="truncate font-medium text-fg">{{ profile.name }}</span>
          <span
            class="rounded border px-1.5 py-0.5 text-xs"
            :class="environmentFor(profile.environment).badgeClass"
          >
            {{ environmentFor(profile.environment).label }}
          </span>
          <span
            v-if="store.isConnected(profile.id)"
            class="truncate text-xs text-success-text"
          >
            Connected · {{ store.statusFor(profile.id)?.server?.version }}
          </span>
          <span
            v-else-if="store.statusFor(profile.id)?.state === 'connecting'"
            class="text-xs text-fg-subtle"
          >
            Connecting…
          </span>
          <span
            v-else-if="store.selectedProfile.value?.id === profile.id"
            class="text-xs text-fg-subtle"
          >
            Selected
          </span>
        </div>
        <span class="truncate font-mono text-sm text-fg-muted">
          {{ profile.username }}@{{ profile.host }}:{{ profile.port }}
          <template v-if="profile.databaseName">
            · {{ profile.databaseName }}
          </template>
        </span>
      </div>
      <BaseButton
        size="sm"
        :disabled="!store.nativeAvailable"
        :loading="busyProfileIds.has(profile.id)"
        @click="toggleConnection(profile)"
      >
        {{ store.isConnected(profile.id) ? "Disconnect" : "Connect" }}
      </BaseButton>
      <BaseButton
        v-if="store.selectedProfile.value?.id !== profile.id"
        size="sm"
        @click="select(profile)"
      >
        Select
      </BaseButton>
      <IconButton
        :icon="Pencil"
        label="Edit Connection"
        @click="store.openEdit(profile.id)"
      />
      <IconButton
        :icon="Copy"
        label="Duplicate Connection"
        @click="duplicate(profile)"
      />
      <IconButton
        :icon="Trash2"
        label="Delete Connection"
        @click="remove(profile)"
      />
    </li>
  </ul>
</template>

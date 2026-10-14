<script setup>
import { Copy, Pencil, Trash2 } from "lucide-vue-next";
import { confirm } from "../../composables/useConfirm.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserMessage } from "../../lib/errors.js";
import { environmentFor } from "../../lib/environments.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";

const store = useConnectionProfiles();
const toasts = useToasts();

async function duplicate(profile) {
  try {
    const warning = await store.duplicate(profile.id);
    if (warning)
      toasts.error("Profile duplicated with a Keychain warning", warning);
    else toasts.success("Connection profile duplicated");
  } catch (error) {
    toasts.error("Couldn't duplicate connection profile", toUserMessage(error));
  }
}

async function remove(profile) {
  const confirmed = await confirm({
    title: `Delete “${profile.name}”?`,
    message:
      "Its saved database and SSH passwords are also removed from Keychain. This can't be undone.",
    confirmLabel: "Delete",
    destructive: true,
  });
  if (!confirmed) return;

  try {
    await store.remove(profile.id);
    toasts.success("Connection profile deleted");
  } catch (error) {
    toasts.error("Couldn't delete connection profile", toUserMessage(error));
  }
}

async function select(profile) {
  try {
    await store.select(profile.id);
  } catch (error) {
    toasts.error("Couldn't select connection profile", toUserMessage(error));
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
            v-if="store.selectedProfile.value?.id === profile.id"
            class="text-xs text-fg-subtle"
          >
            Selected · Not connected
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

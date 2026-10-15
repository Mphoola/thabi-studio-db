<script setup>
import { onMounted, watch } from "vue";
import { ListTree, RefreshCw, Unplug } from "lucide-vue-next";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useExplorer } from "../../composables/useExplorer.js";
import { useHostKeyTrust } from "../../composables/useHostKeyTrust.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail } from "../../lib/errors.js";
import BaseButton from "../common/BaseButton.vue";
import EmptyState from "../common/EmptyState.vue";
import IconButton from "../common/IconButton.vue";
import InlineAlert from "../common/InlineAlert.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";
import ExplorerTree from "./ExplorerTree.vue";

const explorer = useExplorer();
const store = useConnectionProfiles();
const hostKeys = useHostKeyTrust();
const toasts = useToasts();
const { profile, status, connected, rootState } = explorer;

// Load databases whenever a connection becomes available.
watch(
  () => [connected.value, status.value?.connectionId],
  () => {
    explorer.syncConnection();
    if (connected.value && rootState.value.status === "idle") explorer.load();
  },
);
onMounted(() => {
  explorer.syncConnection();
  if (connected.value && rootState.value.status === "idle") explorer.load();
});

async function connect() {
  try {
    await hostKeys.run(() => store.connect(profile.value.id));
  } catch (error) {
    toasts.error(
      `Couldn’t connect to “${profile.value.name}”`,
      toUserDetail(error),
    );
  }
}
</script>

<template>
  <div class="flex h-full flex-col">
    <EmptyState
      v-if="!profile"
      size="sm"
      :icon="ListTree"
      title="Nothing to explore yet"
      description="Choose a connection to browse its databases, schemas and tables."
    />
    <EmptyState
      v-else-if="!connected"
      size="sm"
      :icon="Unplug"
      title="Not connected"
      :description="`Connect to “${profile.name}” to browse its databases.`"
    >
      <BaseButton
        size="sm"
        :loading="status?.state === 'connecting'"
        @click="connect"
      >
        Connect
      </BaseButton>
    </EmptyState>
    <template v-else>
      <div class="flex h-7 shrink-0 items-center justify-between px-3">
        <span class="truncate text-xs text-fg-subtle">
          {{ status.server?.version }}
        </span>
        <IconButton
          :icon="RefreshCw"
          size="sm"
          label="Refresh Explorer"
          shortcut="Mod+R"
          @click="explorer.refresh()"
        />
      </div>
      <div
        v-if="rootState.status === 'loading' && !rootState.children"
        class="flex items-center gap-2 px-3 py-2 text-sm text-fg-subtle"
        role="status"
      >
        <SpinnerIcon :size="12" /> Loading databases…
      </div>
      <div v-else-if="rootState.status === 'error'" class="p-2">
        <InlineAlert
          tone="danger"
          :title="rootState.error?.message ?? 'Couldn’t load databases.'"
          :message="rootState.error?.detail ?? ''"
        >
          <BaseButton
            size="sm"
            @click="
              rootState.error?.kind === 'connection_lost'
                ? connect()
                : explorer.refresh()
            "
          >
            {{
              rootState.error?.kind === "connection_lost"
                ? "Reconnect"
                : "Try Again"
            }}
          </BaseButton>
        </InlineAlert>
      </div>
      <ExplorerTree v-else class="min-h-0 flex-1" />
    </template>
  </div>
</template>

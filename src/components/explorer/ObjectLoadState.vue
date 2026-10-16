<script setup>
import BaseButton from "../common/BaseButton.vue";
import ReconnectButton from "../connections/ReconnectButton.vue";
import InlineAlert from "../common/InlineAlert.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";

/** Reading or failed, for a table tab's data or structure. */
defineProps({
  state: { type: Object, required: true },
  view: { type: String, required: true },
  profileId: { type: String, default: null },
});
defineEmits(["retry"]);
</script>

<template>
  <div v-if="state.status === 'error'" class="p-4">
    <InlineAlert
      tone="danger"
      :title="state.error?.message ?? 'Couldn’t read this object.'"
      :message="state.error?.detail ?? ''"
      mono
    >
      <ReconnectButton
        v-if="['connection_lost', 'not_connected'].includes(state.error?.kind)"
        :profile-id="profileId"
        @reconnected="$emit('retry')"
      />
      <BaseButton v-else size="sm" @click="$emit('retry')"
        >Try Again</BaseButton
      >
    </InlineAlert>
  </div>
  <div
    v-else
    class="flex flex-1 items-center justify-center gap-2 text-fg-subtle"
    role="status"
  >
    <SpinnerIcon :size="14" />
    {{ view === "data" ? "Reading rows…" : "Reading structure…" }}
  </div>
</template>

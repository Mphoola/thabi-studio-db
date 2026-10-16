<script setup>
import { computed } from "vue";
import { useConnectionActions } from "../../composables/useConnectionActions.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import BaseButton from "../common/BaseButton.vue";

/**
 * Reconnects a dropped connection in place. It never re-runs the statement
 * that failed: if a write was in flight its outcome is unknown, so the user
 * decides whether to run it again. Each tab reopens its own session, in its
 * own database, the next time it runs something.
 */
const props = defineProps({ profileId: { type: String, default: null } });
const emit = defineEmits(["reconnected"]);
const store = useConnectionProfiles();
const actions = useConnectionActions();
const profile = computed(
  () => store.profiles.value.find((p) => p.id === props.profileId) ?? null,
);
const busy = computed(() => actions.busy.value.has(props.profileId));

async function reconnect() {
  if (profile.value && (await actions.connect(profile.value)))
    emit("reconnected");
}
</script>

<template>
  <BaseButton v-if="profile" size="sm" :loading="busy" @click="reconnect">
    Reconnect
  </BaseButton>
</template>

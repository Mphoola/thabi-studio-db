<script setup>
import { ref } from "vue";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserMessage } from "../../lib/errors.js";
import BaseButton from "../common/BaseButton.vue";

const props = defineProps({
  profileId: { type: String, default: null },
  disabled: { type: Boolean, default: false },
});
const store = useConnectionProfiles();
const toasts = useToasts();
const testing = ref(false);

async function testConnection() {
  if (!props.profileId) return;
  testing.value = true;
  try {
    await store.test(props.profileId);
    toasts.success("Connection test succeeded");
  } catch (error) {
    toasts.error("Couldn't connect to the database", toUserMessage(error));
  } finally {
    testing.value = false;
  }
}
</script>

<template>
  <BaseButton :disabled="disabled" :loading="testing" @click="testConnection">
    Test Saved Connection
  </BaseButton>
</template>

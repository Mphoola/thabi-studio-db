<script setup>
import { ref } from "vue";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import BaseButton from "../common/BaseButton.vue";

/**
 * Tests the values currently in the form. Emits the outcome so the dialog
 * can show it next to the fields: { ok: true, server, elapsedMs } or
 * { ok: false, error }.
 */
const props = defineProps({
  /** Builds the request from the form; returns null when the form is invalid. */
  buildInput: { type: Function, required: true },
  profileId: { type: String, default: null },
  disabled: { type: Boolean, default: false },
});
const emit = defineEmits(["result"]);
const store = useConnectionProfiles();
const testing = ref(false);

async function testConnection() {
  const input = props.buildInput();
  if (!input) return;
  testing.value = true;
  emit("result", null);
  try {
    const result = await store.test(input, props.profileId);
    emit("result", { ok: true, ...result });
  } catch (error) {
    emit("result", { ok: false, error });
  } finally {
    testing.value = false;
  }
}
</script>

<template>
  <BaseButton :disabled="disabled" :loading="testing" @click="testConnection">
    {{ testing ? "Testing…" : "Test Connection" }}
  </BaseButton>
</template>

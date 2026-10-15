<script setup>
import { computed, ref, watch } from "vue";
import { useSecretPrompt } from "../../composables/useSecretPrompt.js";
import BaseButton from "./BaseButton.vue";
import BaseDialog from "./BaseDialog.vue";
import PasswordInput from "./PasswordInput.vue";

/**
 * Asks for one secret when a connection is set to "Ask every time". Nothing
 * typed here is stored: it is used for this one connection attempt and then
 * goes out of scope.
 */
const { request, settle } = useSecretPrompt();
const value = ref("");

const open = computed({
  get: () => request.value !== null,
  set: (isOpen) => {
    if (!isOpen) settle(null);
  },
});

watch(request, () => (value.value = ""));

function confirm() {
  // An empty secret is a cancellation: connecting with it would only produce
  // a confusing authentication error.
  settle(value.value.length ? value.value : null);
}
</script>

<template>
  <BaseDialog
    v-if="request"
    v-model:open="open"
    size="sm"
    :title="request.title"
    :description="request.message"
  >
    <PasswordInput
      v-model="value"
      class="mt-3"
      :label="request.label"
      autocomplete="off"
      @keydown.enter.prevent="confirm"
    />
    <template #footer>
      <BaseButton @click="settle(null)">Cancel</BaseButton>
      <BaseButton variant="primary" :disabled="!value" @click="confirm">
        Connect
      </BaseButton>
    </template>
  </BaseDialog>
</template>

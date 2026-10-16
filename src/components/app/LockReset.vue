<script setup>
import { ref } from "vue";
import { TriangleAlert } from "lucide-vue-next";
import { useAppLock } from "../../composables/useAppLock.js";
import { toUserMessage } from "../../lib/errors.js";
import { words } from "../../lib/platform.js";
import BaseButton from "../common/BaseButton.vue";
import CheckboxInput from "../common/CheckboxInput.vue";
import InlineAlert from "../common/InlineAlert.vue";

/**
 * The only way past a forgotten app password: everything the password
 * protected is deleted, so resetting it gives nothing away.
 */
const emit = defineEmits(["back"]);
const lock = useAppLock();
const understood = ref(false);
const busy = ref(false);
const error = ref("");

async function reset() {
  busy.value = true;
  error.value = "";
  try {
    await lock.reset();
  } catch (problem) {
    error.value = toUserMessage(problem);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="flex w-full max-w-sm flex-col gap-4">
    <h1 id="lock-title" class="flex items-center gap-1.5 text-lg font-semibold">
      <TriangleAlert
        :size="16"
        :stroke-width="1.75"
        class="text-warning-text"
        aria-hidden="true"
      />
      Reset the app password?
    </h1>
    <p class="text-sm text-fg-muted">
      The password can't be recovered. Resetting it removes what it protects
      from {{ words.computer }}:
    </p>
    <ul class="list-disc pl-5 text-sm text-fg-muted">
      <li>every saved database and SSH password,</li>
      <li>your query history,</li>
      <li>and the app password itself.</li>
    </ul>
    <p class="text-sm text-fg-muted">
      Connections and saved queries stay. You'll type each database password the
      next time you connect, and can set a new app password in Settings.
    </p>
    <CheckboxInput
      v-model="understood"
      label="I understand saved passwords and history will be deleted"
    />
    <InlineAlert
      v-if="error"
      tone="danger"
      title="Couldn't reset"
      :message="error"
    />
    <div class="flex justify-end gap-2">
      <BaseButton :disabled="busy" @click="emit('back')">Back</BaseButton>
      <BaseButton
        variant="danger"
        :disabled="!understood"
        :loading="busy"
        @click="reset"
        >Reset Password</BaseButton
      >
    </div>
  </div>
</template>

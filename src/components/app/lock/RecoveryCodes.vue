<script setup>
import { ref } from "vue";
import { Copy, Download } from "lucide-vue-next";
import { useToasts } from "../../../composables/useToasts.js";
import { copyText } from "../../../lib/clipboard.js";
import { toUserMessage } from "../../../lib/errors.js";
import { saveRecoveryCodes } from "../../../lib/tauri/commands.js";
import { pickRecoveryCodesDestination } from "../../../lib/tauri/files.js";
import BaseButton from "../../common/BaseButton.vue";
import CheckboxInput from "../../common/CheckboxInput.vue";

/**
 * Recovery codes, shown once, like GitHub's. They are not kept anywhere the
 * app can show them again, so the person has to save them now.
 */
const props = defineProps({
  codes: { type: Array, required: true },
});
const saved = defineModel("saved", { type: Boolean, default: false });
const toasts = useToasts();
const wrote = ref(false);

async function copy() {
  try {
    await copyText(props.codes.join("\n"));
    toasts.success("Recovery codes copied");
  } catch (error) {
    toasts.error("Couldn't copy the codes", toUserMessage(error));
  }
}

async function download() {
  try {
    const path = await pickRecoveryCodesDestination();
    if (!path) return;
    await saveRecoveryCodes(path, props.codes);
    wrote.value = true;
    toasts.success("Recovery codes saved");
  } catch (error) {
    toasts.error("Couldn't save the codes", toUserMessage(error));
  }
}
</script>

<template>
  <div class="flex flex-col gap-3">
    <p class="text-sm text-fg-muted">
      If you forget the app password, each code lets you set a new one, once.
      Keep them somewhere safe, away from this computer, such as a password
      manager. You won't see them again.
    </p>
    <ol
      class="grid grid-cols-2 gap-x-6 gap-y-1 rounded-md border border-line bg-surface px-4 py-3 font-mono text-base"
      aria-label="Recovery codes"
    >
      <li v-for="code in codes" :key="code" class="select-all">{{ code }}</li>
    </ol>
    <div class="flex gap-2">
      <BaseButton size="sm" :icon="Copy" @click="copy">Copy</BaseButton>
      <BaseButton size="sm" :icon="Download" @click="download"
        >Save as Text File…</BaseButton
      >
    </div>
    <CheckboxInput
      v-model="saved"
      label="I've saved these codes somewhere safe"
      :description="wrote ? '' : 'Copy or save them first.'"
    />
  </div>
</template>

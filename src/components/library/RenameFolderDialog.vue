<script setup>
import { ref, watch } from "vue";
import { useLibrary } from "../../composables/useLibrary.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserMessage } from "../../lib/errors.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import TextInput from "../common/TextInput.vue";

/** Renames a saved-query folder. An existing name merges the two. */
const open = defineModel("open", { type: Boolean, default: false });
const props = defineProps({ folder: { type: String, default: "" } });
const library = useLibrary();
const toasts = useToasts();
const name = ref("");
const error = ref("");
const saving = ref(false);

watch(open, (value) => {
  if (!value) return;
  name.value = props.folder;
  error.value = "";
});

async function submit() {
  const next = name.value.trim();
  if (!next) {
    error.value = "Enter a folder name.";
    return;
  }
  saving.value = true;
  try {
    if (next !== props.folder) await library.renameFolder(props.folder, next);
    open.value = false;
  } catch (failure) {
    if (failure?.kind === "validation")
      error.value = failure.fields?.folder ?? toUserMessage(failure);
    else toasts.error("Couldn’t rename the folder", toUserMessage(failure));
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <BaseDialog v-model:open="open" size="sm" title="Rename Folder">
    <form
      id="rename-folder-form"
      class="flex flex-col gap-3"
      @submit.prevent="submit"
    >
      <TextInput
        v-model="name"
        label="Name"
        hint="Using another folder’s name merges the two."
        :error="error"
      />
    </form>
    <template #footer>
      <BaseButton @click="open = false">Cancel</BaseButton>
      <BaseButton
        variant="primary"
        type="submit"
        form="rename-folder-form"
        :loading="saving"
      >
        Rename
      </BaseButton>
    </template>
  </BaseDialog>
</template>

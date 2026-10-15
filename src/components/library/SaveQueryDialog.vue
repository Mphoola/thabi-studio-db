<script setup>
import { computed, ref, useId, watch } from "vue";
import { useLibrary } from "../../composables/useLibrary.js";
import { useQuerySave } from "../../composables/useQuerySave.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import TextInput from "../common/TextInput.vue";

/** Names a query when it's first saved, or renames a saved one. */
const open = defineModel("open", { type: Boolean, default: false });
const saver = useQuerySave();
const name = ref("");
const notes = ref("");
const folder = ref("");
const library = useLibrary();
const folderList = useId();
const fields = ref({});
const saving = ref(false);

const editing = computed(() => Boolean(saver.pending.value?.query));

watch(open, (value) => {
  if (!value) return;
  const { tab, query } = saver.pending.value ?? {};
  name.value =
    query?.name ??
    (/^Query \d+$/.test(tab?.title ?? "") ? "" : (tab?.title ?? ""));
  notes.value = query?.notes ?? "";
  folder.value = query?.folder ?? "";
  fields.value = {};
});

async function submit() {
  saving.value = true;
  fields.value =
    (await saver.submit({
      name: name.value,
      notes: notes.value,
      folder: folder.value,
    })) ?? {};
  saving.value = false;
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    size="sm"
    :title="editing ? 'Edit Saved Query' : 'Save Query'"
    description="Saved queries are kept on this computer and listed under Saved."
  >
    <form
      id="save-query-form"
      class="flex flex-col gap-3"
      @submit.prevent="submit"
    >
      <TextInput
        v-model="name"
        label="Name"
        placeholder="Pending orders"
        :error="fields.name"
      />
      <TextInput
        v-model="folder"
        label="Folder"
        placeholder="None"
        hint="Pick a folder or type a new name."
        :list="folderList"
        :error="fields.folder"
      />
      <datalist :id="folderList">
        <option
          v-for="option in library.folders.value"
          :key="option"
          :value="option"
        />
      </datalist>
      <TextInput v-model="notes" label="Notes" placeholder="Optional" />
    </form>
    <template #footer>
      <BaseButton @click="open = false">Cancel</BaseButton>
      <BaseButton
        variant="primary"
        type="submit"
        form="save-query-form"
        :loading="saving"
      >
        {{ editing ? "Save Changes" : "Save Query" }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<script setup>
import { computed, ref, watch } from "vue";
import { useQuerySave } from "../../composables/useQuerySave.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import TextInput from "../common/TextInput.vue";

/** Names a query when it's first saved, or renames a saved one. */
const open = defineModel("open", { type: Boolean, default: false });
const saver = useQuerySave();
const name = ref("");
const notes = ref("");
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
  fields.value = {};
});

async function submit() {
  saving.value = true;
  fields.value =
    (await saver.submit({ name: name.value, notes: notes.value })) ?? {};
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

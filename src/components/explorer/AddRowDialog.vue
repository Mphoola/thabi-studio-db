<script setup>
import { ref, watch } from "vue";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import TextInput from "../common/TextInput.vue";

/** Stages a new row. Blank fields are left out, so the column default applies. */
const open = defineModel("open", { type: Boolean, default: false });
const props = defineProps({
  columns: { type: Array, required: true },
  table: { type: String, default: "" },
});
const emit = defineEmits(["add"]);
const values = ref({});

watch(open, (value) => value && (values.value = {}));

function submit() {
  const row = {};
  for (const column of props.columns) {
    const text = values.value[column.name] ?? "";
    if (text !== "") row[column.name] = text;
  }
  emit("add", row);
  open.value = false;
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    size="md"
    title="Add Row"
    :description="`Staged in “${table}” until you apply. Leave a field blank to use the column’s default.`"
  >
    <form
      id="add-row-form"
      class="flex max-h-[min(55vh,520px)] flex-col gap-3 overflow-y-auto"
      @submit.prevent="submit"
    >
      <TextInput
        v-for="column in columns"
        :key="column.name"
        v-model="values[column.name]"
        :label="column.name"
        :hint="column.typeName"
        placeholder="Default"
        mono
      />
    </form>
    <template #footer>
      <BaseButton @click="open = false">Cancel</BaseButton>
      <BaseButton variant="primary" type="submit" form="add-row-form">
        Add Row
      </BaseButton>
    </template>
  </BaseDialog>
</template>

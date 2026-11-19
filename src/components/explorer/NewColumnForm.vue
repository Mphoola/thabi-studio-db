<script setup>
/**
 * The form for adding a column.
 *
 * Its own component because it is a form, not a table: it holds the half-typed
 * column and the two rules that stop it — a column needs a name, and it cannot
 * take a name the table already has. Both are refused here rather than sent,
 * because the server would refuse them later and the review dialog would
 * already have shown SQL that cannot run.
 */
import { ref } from "vue";
import BaseButton from "../common/BaseButton.vue";
import CheckboxInput from "../common/CheckboxInput.vue";
import TextInput from "../common/TextInput.vue";
import TypeInput from "./TypeInput.vue";

const props = defineProps({
  /** Names already in use, so a duplicate is caught before it is staged. */
  existing: { type: Array, default: () => [] },
  typeNames: { type: Array, required: true },
  /** Defaults to nullable: a NOT NULL column with no default is refused on a
   *  table that has rows, and a new column usually wants to start empty. */
  dataType: { type: String, default: "TEXT" },
});
const emit = defineEmits(["add", "cancel", "error"]);

const blank = (dataType) => ({
  name: "",
  dataType,
  nullable: true,
  default: "",
});
const form = ref(blank(props.dataType));

function submit() {
  const name = form.value.name.trim();
  if (!name) {
    emit("error", "Give the column a name.");
    return;
  }
  if (props.existing.includes(name)) {
    emit("error", `${name} already exists.`);
    return;
  }
  if (!form.value.dataType.trim()) {
    emit("error", `Give ${name} a type.`);
    return;
  }
  emit("add", {
    kind: "addColumn",
    column: {
      name,
      dataType: form.value.dataType.trim(),
      nullable: form.value.nullable,
      // Needed to add a NOT NULL column to a table that already has rows.
      default: form.value.default.trim(),
      comment: "",
      primaryKey: false,
      unique: false,
    },
  });
  form.value = blank(form.value.dataType);
}
</script>

<template>
  <div class="flex flex-wrap items-end gap-2 rounded-md border border-line p-2">
    <TextInput
      v-model="form.name"
      label="Name"
      placeholder="Column name"
      class="w-40"
    />
    <TypeInput
      v-model="form.dataType"
      label="Type"
      :suggestions="typeNames"
      class="w-44"
    />
    <TextInput
      v-model="form.default"
      label="Default"
      mono
      placeholder="none"
      class="w-32"
    />
    <CheckboxInput v-model="form.nullable" label="Nullable" />
    <BaseButton size="sm" variant="primary" @click="submit"
      >Add column</BaseButton
    >
    <BaseButton size="sm" variant="ghost" @click="emit('cancel')"
      >Cancel</BaseButton
    >
  </div>
</template>

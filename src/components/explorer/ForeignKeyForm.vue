<script setup>
/**
 * The form for adding one single-column foreign key.
 *
 * Its own component because it is a form, not a list: it holds its own field
 * state, and the panel above it should not have to know what a field is.
 */
import { computed, ref } from "vue";
import BaseButton from "../common/BaseButton.vue";
import SelectInput from "../common/SelectInput.vue";
import TextInput from "../common/TextInput.vue";

const props = defineProps({
  /** Columns that may carry the key. */
  columns: { type: Array, required: true },
  /** Schema of this table, for building a qualified target. */
  schema: { type: String, default: "" },
  database: { type: String, default: "" },
});
const emit = defineEmits(["add", "cancel"]);

const blank = () => ({
  name: "",
  column: "",
  references: "",
  referencedColumn: "",
  onDelete: "NO ACTION",
  onUpdate: "NO ACTION",
});
const form = ref(blank());

const columnOptions = computed(() =>
  props.columns.map((name) => ({ value: name, label: name })),
);

const ACTIONS = ["NO ACTION", "CASCADE", "SET NULL", "RESTRICT", "SET DEFAULT"];
const actionOptions = ACTIONS.map((value) => ({ value, label: value }));

const ready = computed(() =>
  Boolean(form.value.column && form.value.references),
);

function submit() {
  if (!ready.value) return;
  const column = form.value.column;
  emit("add", {
    kind: "addForeignKey",
    // A blank name is filled in here rather than sent empty: all three
    // engines refuse a constraint with no name.
    name: form.value.name.trim() || `fk_${column}`,
    columns: [column],
    references: [
      props.schema || props.database || "",
      form.value.references,
    ].filter(Boolean),
    referencedColumns: [form.value.referencedColumn.trim() || "id"],
    onDelete: form.value.onDelete,
    onUpdate: form.value.onUpdate,
  });
  form.value = blank();
}
</script>

<template>
  <div class="rounded-md border border-line p-2">
    <div class="flex flex-wrap items-end gap-2">
      <TextInput
        v-model="form.name"
        label="Name"
        placeholder="left blank to suggest"
        class="w-40"
      />
      <SelectInput
        v-model="form.column"
        label="Column"
        :options="columnOptions"
        placeholder="Column"
        class="w-36"
      />
      <TextInput
        v-model="form.references"
        label="References table"
        placeholder="Table"
        class="w-36"
      />
      <TextInput
        v-model="form.referencedColumn"
        label="References column"
        placeholder="id"
        class="w-36"
      />
      <SelectInput
        v-model="form.onDelete"
        label="On delete"
        :options="actionOptions"
      />
      <SelectInput
        v-model="form.onUpdate"
        label="On update"
        :options="actionOptions"
      />
      <BaseButton
        size="sm"
        variant="primary"
        :disabled="!ready"
        @click="submit"
      >
        Add
      </BaseButton>
      <BaseButton size="sm" variant="ghost" @click="emit('cancel')"
        >Cancel</BaseButton
      >
    </div>
    <p class="mt-1.5 text-xs text-fg-subtle">
      Single-column keys. A composite key is not offered, because the pairing
      would be a guess the server would refuse.
    </p>
  </div>
</template>

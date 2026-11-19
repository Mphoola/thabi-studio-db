<script setup>
/**
 * The form for one foreign key, on one or more columns.
 *
 * The referenced table and its columns are picked from the database rather
 * than typed: a typo here is a key the server refuses, and a guessed column
 * (this form used to assume `id`) is a key that points at the wrong thing.
 */
import { computed, onMounted, ref, watch } from "vue";
import { loadCompletionSchema } from "../../composables/useCompletionSchema.js";
import { foreignKeyActions } from "../../lib/structurePlan.js";
import BaseButton from "../common/BaseButton.vue";
import SelectInput from "../common/SelectInput.vue";
import TextInput from "../common/TextInput.vue";
import ColumnListInput from "./ColumnListInput.vue";

const props = defineProps({
  /** Columns of this table that may carry the key. */
  columns: { type: Array, required: true },
  engine: { type: String, default: "postgres" },
  profileId: { type: String, default: "" },
  tabId: { type: String, default: "" },
  schema: { type: String, default: "" },
  database: { type: String, default: "" },
});
const emit = defineEmits(["add", "cancel"]);

const tables = ref([]);
const loading = ref(true);
onMounted(async () => {
  tables.value = await loadCompletionSchema(
    props.profileId,
    props.tabId,
    props.database,
  );
  loading.value = false;
});

const keyOf = (t) => `${t.schema ?? ""}\u0000${t.table}`;
const tableOptions = computed(() => [
  { value: "", label: loading.value ? "Loading tables…" : "Choose a table" },
  ...tables.value.map((t) => ({
    value: keyOf(t),
    label:
      t.schema && t.schema !== props.schema
        ? `${t.schema}.${t.table}`
        : t.table,
  })),
]);

const form = ref({
  name: "",
  columns: [],
  table: "",
  referenced: [],
  onDelete: "NO ACTION",
  onUpdate: "NO ACTION",
});
const target = computed(() =>
  tables.value.find((t) => keyOf(t) === form.value.table),
);
const targetColumns = computed(() =>
  (target.value?.columns ?? []).map((name) => ({ value: name, label: name })),
);
// A new table means new columns to point at; keep the pairing in step with
// the columns chosen on this side.
watch(
  () => [form.value.table, form.value.columns.length],
  () => {
    const names = target.value?.columns ?? [];
    form.value.referenced = form.value.columns.map((column, i) =>
      form.value.referenced[i] && names.includes(form.value.referenced[i])
        ? form.value.referenced[i]
        : names.includes(column)
          ? column
          : "",
    );
  },
);

const actions = computed(() =>
  foreignKeyActions(props.engine).map((value) => ({ value, label: value })),
);
const ready = computed(
  () =>
    form.value.columns.length > 0 &&
    target.value &&
    form.value.referenced.length === form.value.columns.length &&
    form.value.referenced.every(Boolean),
);

function submit() {
  if (!ready.value) return;
  const { schema, table } = target.value;
  emit("add", {
    kind: "addForeignKey",
    // All three engines refuse a constraint with no name.
    name: form.value.name.trim() || `fk_${form.value.columns.join("_")}`,
    columns: form.value.columns,
    references: [schema || props.schema || props.database, table].filter(
      Boolean,
    ),
    referencedColumns: form.value.referenced,
    onDelete: form.value.onDelete,
    onUpdate: form.value.onUpdate,
  });
}
</script>

<template>
  <form
    class="flex flex-col gap-2 rounded-md border border-line p-2"
    @submit.prevent="submit"
  >
    <div class="flex flex-wrap items-end gap-3">
      <TextInput
        v-model="form.name"
        label="Name"
        placeholder="left blank to suggest"
        class="w-44"
      />
      <ColumnListInput v-model="form.columns" :options="columns" />
      <SelectInput
        v-model="form.table"
        label="References"
        :options="tableOptions"
        class="w-48"
      />
    </div>
    <div
      v-if="target && form.columns.length"
      class="flex flex-col gap-1"
      role="group"
      aria-label="Referenced columns"
    >
      <div
        v-for="(column, i) in form.columns"
        :key="column"
        class="flex items-center gap-2 text-sm"
      >
        <span class="w-36 truncate font-mono text-xs">{{ column }}</span>
        <span class="text-fg-subtle">→</span>
        <SelectInput
          v-model="form.referenced[i]"
          class="w-44"
          :options="[{ value: '', label: 'Column…' }, ...targetColumns]"
          :aria-label="`Column in ${target.table} for ${column}`"
        />
      </div>
    </div>
    <div class="flex flex-wrap items-end gap-3">
      <SelectInput
        v-model="form.onDelete"
        label="On delete"
        :options="actions"
      />
      <SelectInput
        v-model="form.onUpdate"
        label="On update"
        :options="actions"
      />
      <div class="flex gap-1.5">
        <BaseButton size="sm" type="submit" variant="primary" :disabled="!ready"
          >Add Key</BaseButton
        >
        <BaseButton size="sm" variant="ghost" @click="emit('cancel')"
          >Cancel</BaseButton
        >
      </div>
    </div>
  </form>
</template>

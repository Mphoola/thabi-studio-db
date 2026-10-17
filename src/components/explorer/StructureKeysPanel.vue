<script setup>
/**
 * Indexes and foreign keys, listed and — while editing — changed.
 *
 * The listing is shared with the read-only Structure view rather than written
 * twice, because two renderers of the same keys would eventually disagree
 * about what a table has.
 *
 * Foreign keys here are single-column, which is what `describe` reports. A
 * composite key is not offered: the UI would have to guess a pairing the
 * server would then refuse.
 */
import { computed, ref } from "vue";
import { KeyRound, Link2, Plus, Trash2 } from "lucide-vue-next";
import BaseButton from "../common/BaseButton.vue";
import CheckboxInput from "../common/CheckboxInput.vue";
import ForeignKeyForm from "./ForeignKeyForm.vue";
import IconButton from "../common/IconButton.vue";
import SelectInput from "../common/SelectInput.vue";
import TextInput from "../common/TextInput.vue";

const props = defineProps({
  structure: { type: Object, required: true },
  /** Only set while the Structure tab is being edited. */
  editing: { type: Boolean, default: false },
});
const emit = defineEmits(["change"]);

const addingIndex = ref(false);
const addingKey = ref(false);

const indexColumns = computed(() =>
  (props.structure.columns ?? []).map((column) => column.name),
);
const primaryKey = computed(
  () => (props.structure.indexes ?? []).find((index) => index.primary) ?? null,
);
/** Columns that are not the key, since a key cannot reference itself here. */
const keyColumns = computed(() =>
  indexColumns.value.filter((name) => primaryKey.value?.columns?.[0] !== name),
);
const keyNames = computed(() =>
  keyColumns.value.map((name) => ({ value: name, label: name })),
);

const newIndex = ref({ name: "", column: "", unique: true });
function addIndex() {
  if (!newIndex.value.column) return;
  emit("change", {
    kind: "addIndex",
    name: newIndex.value.name.trim() || suggestedIndexName(),
    columns: [newIndex.value.column],
    unique: newIndex.value.unique,
  });
  addingIndex.value = false;
  newIndex.value = { name: "", column: "", unique: true };
}

/**
 * MySQL will not create an index with no name, and a name made up here is
 * only a suggestion — the pattern is the engine's, not a guess about what
 * someone meant.
 */
function suggestedIndexName() {
  const column = newIndex.value.column;
  if (!column) return "index";
  return newIndex.value.unique ? `uq_${column}` : `idx_${column}`;
}
</script>

<template>
  <section class="mt-4">
    <h3
      class="mb-1.5 flex items-center gap-1.5 text-xs font-semibold text-fg-muted"
    >
      <KeyRound :size="14" :stroke-width="1.75" aria-hidden="true" />
      Keys and indexes
    </h3>

    <table class="w-full text-sm">
      <thead class="text-left text-xs text-fg-subtle">
        <tr>
          <th class="py-1 pr-2 font-normal">Name</th>
          <th class="py-1 pr-2 font-normal">Columns</th>
          <th class="py-1 pr-2 font-normal">Kind</th>
          <th v-if="editing" class="w-8 py-1" />
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="index in structure.indexes ?? []"
          :key="index.name"
          class="border-t border-line"
        >
          <td class="py-1 pr-2 font-mono text-xs">{{ index.name }}</td>
          <td class="py-1 pr-2 text-xs text-fg-muted">
            {{ index.columns?.length ? index.columns.join(", ") : "—" }}
          </td>
          <td class="py-1 pr-2 text-xs text-fg-muted">
            {{
              index.primary ? "Primary key" : index.unique ? "Unique" : "Index"
            }}
          </td>
          <td v-if="editing" class="py-1 text-right">
            <IconButton
              v-if="!index.primary"
              :icon="Trash2"
              :label="`Drop index ${index.name}`"
              @click="emit('change', { kind: 'dropIndex', name: index.name })"
            />
          </td>
        </tr>
        <tr v-if="!structure.indexes?.length">
          <td colspan="4" class="py-1.5 text-sm text-fg-subtle">No indexes.</td>
        </tr>
      </tbody>
    </table>

    <div v-if="editing" class="mt-1.5 space-y-2">
      <button
        v-if="!addingIndex"
        type="button"
        class="flex items-center gap-1 text-xs text-fg-muted hover:text-fg"
        @click="addingIndex = true"
      >
        <Plus :size="13" :stroke-width="1.75" aria-hidden="true" />
        Add index
      </button>
      <div v-else class="rounded-md border border-line p-2">
        <div class="flex items-end gap-2">
          <TextInput
            v-model="newIndex.name"
            label="Name"
            placeholder="left blank to suggest"
            class="w-40"
          />
          <SelectInput
            v-model="newIndex.column"
            label="Column"
            :options="keyNames"
            placeholder="Column"
            class="w-40"
          />
          <CheckboxInput v-model="newIndex.unique" label="Unique" />
          <BaseButton size="sm" variant="primary" @click="addIndex"
            >Add</BaseButton
          >
          <BaseButton size="sm" variant="ghost" @click="addingIndex = false"
            >Cancel</BaseButton
          >
        </div>
      </div>
    </div>

    <div class="mt-4">
      <h4
        class="mb-1.5 flex items-center gap-1.5 text-xs font-semibold text-fg-muted"
      >
        <Link2 :size="14" :stroke-width="1.75" aria-hidden="true" />
        Foreign keys
      </h4>
      <p
        v-if="!structure.columns?.some((column) => column.references)"
        class="text-sm text-fg-subtle"
      >
        No foreign keys.
      </p>
      <ul v-else class="space-y-0.5 text-sm">
        <li
          v-for="column in structure.columns.filter((one) => one.references)"
          :key="column.name"
          class="flex items-center gap-1.5 text-xs"
        >
          <span class="font-mono">{{ column.name }}</span>
          <span class="text-fg-subtle">→</span>
          <span class="text-fg-muted">
            {{ column.references.schema ? `${column.references.schema}.` : ""
            }}{{ column.references.table }}.{{ column.references.column }}
          </span>
        </li>
      </ul>

      <div v-if="editing" class="mt-1.5">
        <button
          v-if="!addingKey"
          type="button"
          class="flex items-center gap-1 text-xs text-fg-muted hover:text-fg"
          @click="addingKey = true"
        >
          <Plus :size="13" :stroke-width="1.75" aria-hidden="true" />
          Add foreign key
        </button>
        <ForeignKeyForm
          v-else
          :columns="keyColumns"
          :schema="structure.schema"
          :database="structure.database"
          @add="emit('change', $event)"
          @cancel="addingKey = false"
        />
      </div>
    </div>
  </section>
</template>

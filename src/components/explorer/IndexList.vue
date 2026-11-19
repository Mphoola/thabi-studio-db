<script setup>
import { computed, ref } from "vue";
import { KeyRound, Plus, Trash2 } from "lucide-vue-next";
import BaseButton from "../common/BaseButton.vue";
import CheckboxInput from "../common/CheckboxInput.vue";
import IconButton from "../common/IconButton.vue";
import TextInput from "../common/TextInput.vue";
import ColumnListInput from "./ColumnListInput.vue";

/**
 * A table's indexes and primary key, listed, and while editing, staged:
 * indexes on one or more columns, and the primary key set or dropped by the
 * name the server gave it.
 */
const props = defineProps({
  structure: { type: Object, required: true },
  editing: { type: Boolean, default: false },
});
const emit = defineEmits(["change"]);

const columns = computed(() =>
  (props.structure.columns ?? []).map((column) => column.name),
);
const primaryKey = computed(
  () => (props.structure.indexes ?? []).find((index) => index.primary) ?? null,
);

/** "index" or "primary" while a form is open. */
const adding = ref(null);
const form = ref({ name: "", columns: [], unique: false });

function open(kind) {
  adding.value = kind;
  form.value = { name: "", columns: [], unique: false };
}

/** A name in the engines' usual pattern, only when none was typed: MySQL
 *  refuses an index without one. */
function suggested() {
  const joined = form.value.columns.join("_");
  if (adding.value === "primary") return `pk_${props.structure.name ?? joined}`;
  return `${form.value.unique ? "uq" : "idx"}_${joined}`;
}

function submit() {
  if (!form.value.columns.length) return;
  const name = form.value.name.trim() || suggested();
  emit(
    "change",
    adding.value === "primary"
      ? { kind: "setPrimaryKey", name, columns: form.value.columns }
      : {
          kind: "addIndex",
          name,
          columns: form.value.columns,
          unique: form.value.unique,
        },
  );
  adding.value = null;
}

function drop(index) {
  emit(
    "change",
    index.primary
      ? { kind: "dropPrimaryKey", name: index.name }
      : { kind: "dropIndex", name: index.name },
  );
}
</script>

<template>
  <section>
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
              :icon="Trash2"
              :label="
                index.primary
                  ? 'Drop the primary key'
                  : `Drop index ${index.name}`
              "
              @click="drop(index)"
            />
          </td>
        </tr>
        <tr v-if="!structure.indexes?.length">
          <td colspan="4" class="py-1.5 text-sm text-fg-subtle">No indexes.</td>
        </tr>
      </tbody>
    </table>

    <div v-if="editing" class="mt-1.5">
      <div v-if="!adding" class="flex gap-3">
        <button
          type="button"
          class="flex items-center gap-1 text-xs text-fg-muted hover:text-fg"
          @click="open('index')"
        >
          <Plus :size="13" :stroke-width="1.75" aria-hidden="true" />
          Add index
        </button>
        <button
          v-if="!primaryKey"
          type="button"
          class="flex items-center gap-1 text-xs text-fg-muted hover:text-fg"
          @click="open('primary')"
        >
          <Plus :size="13" :stroke-width="1.75" aria-hidden="true" />
          Set primary key
        </button>
      </div>
      <form
        v-else
        class="flex flex-wrap items-end gap-3 rounded-md border border-line p-2"
        @submit.prevent="submit"
      >
        <TextInput
          v-model="form.name"
          label="Name"
          :placeholder="suggested()"
          class="w-44"
        />
        <ColumnListInput v-model="form.columns" :options="columns" />
        <CheckboxInput
          v-if="adding === 'index'"
          v-model="form.unique"
          label="Unique"
        />
        <div class="flex gap-1.5">
          <BaseButton
            size="sm"
            type="submit"
            variant="primary"
            :disabled="!form.columns.length"
            >{{ adding === "primary" ? "Set Key" : "Add Index" }}</BaseButton
          >
          <BaseButton size="sm" variant="ghost" @click="adding = null"
            >Cancel</BaseButton
          >
        </div>
      </form>
    </div>
  </section>
</template>

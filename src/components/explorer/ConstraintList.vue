<script setup>
import { computed, ref } from "vue";
import { Link2, Plus, ShieldCheck, Trash2 } from "lucide-vue-next";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";
import TextInput from "../common/TextInput.vue";
import ForeignKeyForm from "./ForeignKeyForm.vue";

/**
 * A table's foreign keys and check constraints, listed by the names the
 * server knows them by, so either can be dropped; and, while editing, added.
 */
const props = defineProps({
  structure: { type: Object, required: true },
  editing: { type: Boolean, default: false },
  engine: { type: String, default: "postgres" },
  profileId: { type: String, default: "" },
  tabId: { type: String, default: "" },
  schema: { type: String, default: "" },
  database: { type: String, default: "" },
});
const emit = defineEmits(["change"]);

const constraints = computed(() => props.structure.constraints ?? null);
/** Foreign keys by name; an older describe without constraints falls back to
 *  the single-column references on the columns, which can't be dropped. */
const foreignKeys = computed(() =>
  constraints.value
    ? constraints.value.filter((c) => c.kind === "foreignKey")
    : (props.structure.columns ?? [])
        .filter((column) => column.references)
        .map((column) => ({
          name: "",
          columns: [column.name],
          referencedSchema: column.references.schema,
          referencedTable: column.references.table,
          referencedColumns: [column.references.column],
        })),
);
const checks = computed(() =>
  (constraints.value ?? []).filter((c) => c.kind === "check"),
);
const columnNames = computed(() =>
  (props.structure.columns ?? []).map((column) => column.name),
);

const addingKey = ref(false);
const addingCheck = ref(false);
const check = ref({ name: "", expression: "" });

function addKey(change) {
  emit("change", change);
  addingKey.value = false;
}

function addCheck() {
  const expression = check.value.expression.trim();
  if (!expression) return;
  emit("change", {
    kind: "addCheck",
    name: check.value.name.trim() || `chk_${props.structure.name ?? "table"}`,
    expression,
  });
  check.value = { name: "", expression: "" };
  addingCheck.value = false;
}

const target = (key) =>
  `${key.referencedSchema && key.referencedSchema !== props.schema ? `${key.referencedSchema}.` : ""}${key.referencedTable}`;
</script>

<template>
  <section class="flex flex-col gap-4">
    <div>
      <h4
        class="mb-1.5 flex items-center gap-1.5 text-xs font-semibold text-fg-muted"
      >
        <Link2 :size="14" :stroke-width="1.75" aria-hidden="true" />
        Foreign keys
      </h4>
      <p v-if="!foreignKeys.length" class="text-sm text-fg-subtle">
        No foreign keys.
      </p>
      <ul v-else class="flex flex-col gap-0.5 text-xs">
        <li
          v-for="key in foreignKeys"
          :key="key.name || key.columns.join()"
          class="flex items-center gap-1.5"
        >
          <span v-if="key.name" class="font-mono text-fg-subtle">{{
            key.name
          }}</span>
          <span class="font-mono">{{ key.columns.join(", ") }}</span>
          <span class="text-fg-subtle">→</span>
          <span class="text-fg-muted"
            >{{ target(key) }} ({{ key.referencedColumns.join(", ") }})</span
          >
          <span
            v-if="key.onDelete && key.onDelete !== 'NO ACTION'"
            class="text-fg-subtle"
            >on delete {{ key.onDelete.toLowerCase() }}</span
          >
          <IconButton
            v-if="editing && key.name"
            class="ml-auto"
            :icon="Trash2"
            :label="`Drop foreign key ${key.name}`"
            @click="emit('change', { kind: 'dropForeignKey', name: key.name })"
          />
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
          :columns="columnNames"
          :engine="engine"
          :profile-id="profileId"
          :tab-id="tabId"
          :schema="schema"
          :database="database"
          @add="addKey"
          @cancel="addingKey = false"
        />
      </div>
    </div>

    <div v-if="constraints">
      <h4
        class="mb-1.5 flex items-center gap-1.5 text-xs font-semibold text-fg-muted"
      >
        <ShieldCheck :size="14" :stroke-width="1.75" aria-hidden="true" />
        Checks
      </h4>
      <p v-if="!checks.length" class="text-sm text-fg-subtle">No checks.</p>
      <ul v-else class="flex flex-col gap-0.5 text-xs">
        <li
          v-for="one in checks"
          :key="one.name"
          class="flex items-center gap-1.5"
        >
          <span class="font-mono text-fg-subtle">{{ one.name }}</span>
          <span class="truncate font-mono">{{ one.definition }}</span>
          <IconButton
            v-if="editing"
            class="ml-auto"
            :icon="Trash2"
            :label="`Drop check ${one.name}`"
            @click="emit('change', { kind: 'dropCheck', name: one.name })"
          />
        </li>
      </ul>
      <div v-if="editing" class="mt-1.5">
        <button
          v-if="!addingCheck"
          type="button"
          class="flex items-center gap-1 text-xs text-fg-muted hover:text-fg"
          @click="addingCheck = true"
        >
          <Plus :size="13" :stroke-width="1.75" aria-hidden="true" />
          Add check
        </button>
        <form
          v-else
          class="flex flex-wrap items-end gap-3 rounded-md border border-line p-2"
          @submit.prevent="addCheck"
        >
          <TextInput v-model="check.name" label="Name" class="w-44" />
          <TextInput
            v-model="check.expression"
            label="Condition"
            mono
            placeholder="total >= 0"
            class="min-w-60 flex-1"
          />
          <div class="flex gap-1.5">
            <BaseButton
              size="sm"
              type="submit"
              variant="primary"
              :disabled="!check.expression.trim()"
              >Add Check</BaseButton
            >
            <BaseButton size="sm" variant="ghost" @click="addingCheck = false"
              >Cancel</BaseButton
            >
          </div>
        </form>
      </div>
    </div>
  </section>
</template>

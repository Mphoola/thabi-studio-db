<script setup>
import { computed, ref, watch } from "vue";
import { Minus, Plus } from "lucide-vue-next";
import { applyTableDdl, previewTableDdl } from "../../lib/tauri/commands.js";
import { toUserMessage } from "../../lib/errors.js";
import { useToasts } from "../../composables/useToasts.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import IconButton from "../common/IconButton.vue";
import InlineAlert from "../common/InlineAlert.vue";
import SelectInput from "../common/SelectInput.vue";
import TextInput from "../common/TextInput.vue";
import DdlReviewDialog from "./DdlReviewDialog.vue";

/**
 * Creates a table from a name and a few columns.
 *
 * The wizard stops at what it can do honestly: a name and columns, with types,
 * nullability and a primary key. Anything it cannot express is not offered
 * rather than offered and ignored — keys, indexes and foreign keys come from
 * the designer once the table exists, where there is a structure to read.
 *
 * Nothing is sent until the SQL has been shown and agreed to, and the SQL shown
 * is the SQL that runs.
 */
const props = defineProps({
  /** Connection the table goes in. */
  profileId: { type: String, required: true },
  tabId: { type: String, required: true },
  database: { type: String, required: true },
  schema: { type: String, default: "" },
  /** The driver, so the offered type names are ones it knows. */
  engine: { type: String, default: "postgres" },
});
const emit = defineEmits(["created"]);

const open = defineModel("open", { type: Boolean, default: false });
const toasts = useToasts();

/** Offered types, in each engine's own words, filtered by what the engine has. */
const TYPES = {
  postgres: [
    "INTEGER",
    "BIGINT",
    "TEXT",
    "VARCHAR(255)",
    "NUMERIC(12,2)",
    "BOOLEAN",
    "TIMESTAMPTZ",
  ],
  mysql: [
    "INT",
    "BIGINT",
    "TEXT",
    "VARCHAR(255)",
    "DECIMAL(12,2)",
    "BOOLEAN",
    "DATETIME",
  ],
  sqlserver: [
    "INT",
    "BIGINT",
    "NVARCHAR(MAX)",
    "VARCHAR(255)",
    "DECIMAL(12,2)",
    "BIT",
    "DATETIME2",
  ],
};
const driver = computed(() => props.engine ?? TYPES.postgres);
const types = computed(() => TYPES[driver.value] ?? TYPES.postgres);

const name = ref("");
const rows = ref([
  { name: "id", type: "INTEGER", nullable: false, key: true },
  { name: "", type: "TEXT", nullable: true, key: false },
]);
const error = ref("");
const previewing = ref(false);
const applying = ref(false);
const preview = ref(null);
const reviewing = ref(false);

watch(open, (value) => {
  if (!value) return;
  name.value = "";
  rows.value = [
    { name: "id", type: types.value[0], nullable: false, key: true },
    { name: "", type: types.value[2] ?? "TEXT", nullable: true, key: false },
  ];
  preview.value = null;
  error.value = "";
});

/** A row the server would accept: a name, a type, and a key exactly once. */
function usable() {
  const named = rows.value.filter((row) => row.name.trim());
  if (!named.length) return null;
  const keys = named.filter((row) => row.key);
  if (keys.length !== 1) return null;
  return named;
}

const plan = computed(() => {
  const named = usable();
  if (!named || !name.value.trim()) return null;
  // A create plan, not a list of alters: the table is made here, and the
  // columns it is made with are the change.
  return {
    create: {
      columns: named.map((row) => ({
        name: row.name.trim(),
        dataType: row.type,
        nullable: row.nullable && !row.key,
        primaryKey: row.key,
      })),
    },
  };
});

const canReview = computed(() => Boolean(plan.value));

async function review() {
  if (!plan.value) {
    error.value = name.value.trim()
      ? "Give the table one primary key column."
      : "Give the table a name.";
    return;
  }
  error.value = "";
  previewing.value = true;
  try {
    preview.value = await previewTableDdl(
      props.profileId,
      props.tabId,
      {
        database: props.database,
        schema: props.schema || null,
        name: name.value.trim(),
      },
      plan.value,
    );
    reviewing.value = true;
  } catch (failure) {
    error.value = toUserMessage(failure);
  } finally {
    previewing.value = false;
  }
}

async function run() {
  applying.value = true;
  try {
    await applyTableDdl(
      props.profileId,
      props.tabId,
      {
        database: props.database,
        schema: props.schema || null,
        name: name.value.trim(),
      },
      plan.value,
    );
    toasts.success(`Created “${name.value.trim()}”`);
    reviewing.value = false;
    open.value = false;
    emit("created", name.value.trim());
  } catch (failure) {
    toasts.error("Couldn't create the table", toUserMessage(failure));
  } finally {
    applying.value = false;
  }
}

const heading = computed(() =>
  [props.schema || props.database, name.value.trim()].filter(Boolean).join("."),
);
</script>

<template>
  <BaseDialog v-model:open="open" size="lg" title="Create Table">
    <form class="flex flex-col gap-3" @submit.prevent="review">
      <TextInput
        v-model="name"
        label="Table name"
        placeholder="orders"
        aria-label="Table name"
        autocomplete="off"
      />

      <div class="flex flex-col gap-1.5">
        <span class="text-sm font-medium text-fg">Columns</span>
        <div
          v-for="(row, index) in rows"
          :key="index"
          class="flex items-center gap-2"
        >
          <TextInput
            v-model="row.name"
            class="min-w-0 flex-1"
            placeholder="name"
            :aria-label="`Column ${index + 1} name`"
            autocomplete="off"
          />
          <SelectInput
            v-model="row.type"
            class="w-44 shrink-0"
            :options="types.map((type) => ({ value: type, label: type }))"
            :aria-label="`Column ${index + 1} type`"
          />
          <label class="flex items-center gap-1.5 text-xs text-fg-muted">
            <input
              v-model="row.nullable"
              type="checkbox"
              :disabled="row.key"
              :aria-label="`Column ${index + 1} nullable`"
            />
            Null
          </label>
          <label class="flex items-center gap-1.5 text-xs text-fg-muted">
            <input
              :checked="row.key"
              type="radio"
              name="primary-key"
              :aria-label="`Column ${index + 1} is the primary key`"
              @change="
                rows.forEach((one) => (one.key = false));
                row.key = true;
              "
            />
            Key
          </label>
          <IconButton
            :icon="Minus"
            size="sm"
            label="Remove Column"
            :disabled="rows.length <= 1"
            @click="rows = rows.filter((_, i) => i !== index)"
          />
        </div>
        <div>
          <BaseButton
            size="sm"
            variant="ghost"
            :icon="Plus"
            @click="
              rows.push({
                name: '',
                type: types[2],
                nullable: true,
                key: false,
              })
            "
          >
            Add column
          </BaseButton>
        </div>
      </div>

      <InlineAlert
        v-if="error"
        tone="danger"
        title="Not quite"
        :message="error"
      />
      <p class="text-xs text-fg-muted">
        Indexes, foreign keys and checks are added from the table's structure
        once it exists.
      </p>
    </form>
    <template #footer>
      <BaseButton size="sm" @click="open = false">Cancel</BaseButton>
      <BaseButton
        size="sm"
        variant="primary"
        :loading="previewing"
        :disabled="!canReview"
        @click="review"
      >
        Review SQL
      </BaseButton>
    </template>
  </BaseDialog>

  <DdlReviewDialog
    v-model:open="reviewing"
    :title="`Create ${heading}`"
    :preview="preview"
    :applying="applying"
    @close="reviewing = false"
    @apply="run"
  />
</template>

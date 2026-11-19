<script setup>
import { computed, ref, watch } from "vue";
import { applyTableDdl, previewTableDdl } from "../../lib/tauri/commands.js";
import { toUserMessage } from "../../lib/errors.js";
import { useToasts } from "../../composables/useToasts.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import InlineAlert from "../common/InlineAlert.vue";
import TextInput from "../common/TextInput.vue";
import DdlReviewDialog from "./DdlReviewDialog.vue";
import CreateTableColumns from "./CreateTableColumns.vue";
import {
  createPlan,
  createProblem,
  startingRows,
  typesFor,
} from "../../lib/createTable.js";

/**
 * Creates a table from a name and a few columns.
 *
 * The wizard stops at what it can do honestly: a name and columns, with types,
 * defaults, nullability and a primary key on one or more columns. Anything it cannot express is not offered
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

const types = computed(() => typesFor(props.engine));

const name = ref("");
const rows = ref(startingRows(props.engine));
const error = ref("");
const previewing = ref(false);
const applying = ref(false);
const preview = ref(null);
const reviewing = ref(false);

watch(open, (value) => {
  if (!value) return;
  name.value = "";
  rows.value = startingRows(props.engine);
  preview.value = null;
  error.value = "";
});

// A create plan, not a list of alters: the table is made here, and the
// columns it is made with are the change.
const plan = computed(() => createPlan(name.value, rows.value));
const canReview = computed(() => Boolean(plan.value));

async function review() {
  if (!plan.value) {
    error.value = createProblem(name.value, rows.value);
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

      <CreateTableColumns v-model="rows" :engine="engine" :types="types" />

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

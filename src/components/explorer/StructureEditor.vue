<script setup>
import { computed, ref } from "vue";
import { Pencil, Plus } from "lucide-vue-next";
import { useStructureChanges } from "../../composables/useStructureChanges.js";
import BaseButton from "../common/BaseButton.vue";
import InlineAlert from "../common/InlineAlert.vue";
import DdlReviewDialog from "./DdlReviewDialog.vue";
import StagedChanges from "./StagedChanges.vue";
import NewColumnForm from "./NewColumnForm.vue";
import StructureColumnsTable from "./StructureColumnsTable.vue";
import StructureKeysPanel from "./StructureKeysPanel.vue";

/**
 * Edits a table, staged until they are reviewed.
 *
 * Nothing is sent from here. A change is recorded, the SQL is worked out and
 * shown, and only then is anything run — the same path the Create Table wizard
 * uses, so there is one place where SQL becomes a database change.
 *
 * Edits live on the tab rather than here, so switching tabs and coming back
 * does not silently throw away what was being changed.
 *
 * Columns are in StructureColumnsTable, keys and indexes in
 * StructureKeysPanel. Both stage into the same list: one review dialog shows
 * the lot, and one apply runs it in order.
 */
const props = defineProps({
  tab: { type: Object, required: true },
  structure: { type: Object, required: true },
  profileId: { type: String, required: true },
  database: { type: String, default: "" },
  schema: { type: String, default: "" },
  title: { type: String, default: "" },
});
const emit = defineEmits(["applied"]);

const changes = useStructureChanges({
  tab: props.tab,
  structure: computed(() => props.structure),
  target: computed(() => ({
    database: props.database,
    schema: props.schema,
    name: props.title,
  })),
  profileId: computed(() => props.profileId),
  onApplied: () => emit("applied"),
});

/** Nullability needs the column's type carried: two engines restate the whole
 *  column to change it, and a restatement with no type changes the type. */
function toggleNullable(column) {
  changes.stage({
    kind: "setNullable",
    name: column.name,
    nullable: !column.nullable,
    dataType: column.dataType,
  });
}

function dropColumn(column) {
  changes.stage({ kind: "dropColumn", name: column.name });
}

const editing = ref(false);
const adding = ref(false);

/** Types this table already uses, so a new column tends to match its table. */
const existingTypes = computed(() => [
  ...new Set(props.structure.columns.map((column) => column.dataType)),
]);

const existingNames = computed(() =>
  props.structure.columns.map((column) => column.name),
);

/** The same types as a space-separated list, for the column type datalist. */
const typeNames = computed(() => existingTypes.value.join(" "));

/**
 * Staged as it is typed. `stageChange` replaces a change on the same column,
 * so typing VARCHAR(255) is one staged change rather than eleven.
 */
function rename(column, name) {
  const to = String(name).trim();
  if (!to || to === column.name) return;
  changes.stage({ kind: "renameColumn", from: column.name, to });
}

function retype(column, dataType) {
  const to = String(dataType).trim();
  if (!to || to === column.dataType) return;
  changes.stage({ kind: "setType", name: column.name, dataType: to });
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <div class="flex items-center gap-2 border-b border-line px-3 py-1.5">
      <BaseButton
        size="sm"
        :icon="Pencil"
        :variant="editing ? 'primary' : 'ghost'"
        @click="editing = !editing"
      >
        {{ editing ? "Done editing" : "Edit structure" }}
      </BaseButton>
      <template v-if="editing">
        <BaseButton
          size="sm"
          variant="ghost"
          :icon="Plus"
          @click="adding = !adding"
        >
          Add column
        </BaseButton>
      </template>
      <!-- Staged changes stay reachable whether or not the editor is open.
           Hiding them behind "Done" is how staged work gets lost. -->
      <template v-if="changes.dirty.value">
        <span class="text-xs text-fg-muted">
          {{ changes.edits.value.length }} staged
          {{ changes.edits.value.length === 1 ? "change" : "changes" }}
          {{ editing ? "" : "— review and run them to save" }}
        </span>
        <BaseButton
          size="sm"
          class="ml-auto"
          variant="primary"
          :loading="changes.previewing.value"
          :title="
            editing ? 'Show the SQL, then run it' : 'Show the SQL, then run it'
          "
          @click="changes.review"
        >
          {{ editing ? "Review SQL" : "Review and save" }}
        </BaseButton>
        <BaseButton size="sm" variant="ghost" @click="changes.discard">
          Discard
        </BaseButton>
      </template>
    </div>

    <InlineAlert
      v-if="changes.error.value"
      tone="danger"
      title="Not quite"
      :message="changes.error.value"
    />

    <NewColumnForm
      v-if="adding"
      class="mx-3 my-2"
      :existing="existingNames"
      :type-names="existingTypes"
      @add="changes.stage"
      @error="changes.error.value = $event"
      @cancel="adding = false"
    />

    <StagedChanges
      v-if="changes.dirty.value"
      :edits="changes.edits.value"
      :describe="changes.describeEdit"
      :warnings="changes.warnings.value"
      @undo="changes.undo"
      @discard="changes.discard"
    />

    <div class="selectable overflow-auto p-4">
      <StructureColumnsTable
        :columns="structure.columns"
        :editing="editing"
        :type-names="typeNames"
        @rename="rename"
        @retype="retype"
        @toggle-nullable="toggleNullable"
        @drop="dropColumn"
      />
      <StructureKeysPanel
        :structure="structure"
        :editing="editing"
        :schema="schema"
        :database="database"
        @change="changes.stage"
      />
    </div>

    <DdlReviewDialog
      v-model:open="changes.reviewOpen.value"
      :title="`Change ${title}`"
      :preview="changes.preview.value"
      :applying="changes.applying.value"
      @close="changes.reviewOpen.value = false"
      @apply="changes.run"
    />
  </div>
</template>

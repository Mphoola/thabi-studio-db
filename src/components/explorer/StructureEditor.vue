<script setup>
import { computed, ref } from "vue";
import { Pencil, Plus, X } from "lucide-vue-next";
import { useStructureChanges } from "../../composables/useStructureChanges.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";
import InlineAlert from "../common/InlineAlert.vue";
import SelectInput from "../common/SelectInput.vue";
import TextInput from "../common/TextInput.vue";
import DdlReviewDialog from "./DdlReviewDialog.vue";
import StagedChanges from "./StagedChanges.vue";
import StructureColumnsTable from "./StructureColumnsTable.vue";

/**
 * Edits a table's columns and indexes, staged until they are reviewed.
 *
 * Nothing is sent from here. A change is recorded, the SQL is worked out and
 * shown, and only then is anything run — the same path the Create Table wizard
 * uses, so there is one place where SQL becomes a database change.
 *
 * Edits live on the tab rather than here, so switching tabs and coming back
 * does not silently throw away what was being changed.
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

const editing = ref(false);
const adding = ref(false);
const newColumn = ref({ name: "", dataType: "TEXT", nullable: true });

/** Types this table already uses, for a new column to match. */
const types = computed(() => {
  const seen = [
    ...new Set(props.structure.columns.map((column) => column.dataType)),
  ];
  return [
    { value: newColumn.value.dataType, label: newColumn.value.dataType },
    ...seen
      .filter((type) => type !== newColumn.value.dataType)
      .map((type) => ({
        value: type,
        label: type,
      })),
  ];
});

/** Types this table already uses, offered as the input's suggestions. */
const typeNames = computed(() =>
  [...new Set(props.structure.columns.map((column) => column.dataType))].join(
    " ",
  ),
);

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

function addColumn() {
  const name = newColumn.value.name.trim();
  if (!name) {
    changes.error.value = "Give the column a name.";
    return;
  }
  if (props.structure.columns.some((column) => column.name === name)) {
    changes.error.value = `${name} already exists.`;
    return;
  }
  changes.stage({
    kind: "addColumn",
    column: {
      name,
      dataType: newColumn.value.dataType,
      nullable: newColumn.value.nullable,
      default: "",
      comment: "",
      primaryKey: false,
      unique: false,
    },
  });
  newColumn.value = {
    name: "",
    dataType: newColumn.value.dataType,
    nullable: true,
  };
  adding.value = false;
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

    <div
      v-if="adding"
      class="flex items-center gap-2 border-b border-line px-3 py-2"
    >
      <TextInput
        v-model="newColumn.name"
        class="min-w-0 flex-1"
        placeholder="column_name"
        aria-label="New column name"
        autocomplete="off"
      />
      <SelectInput
        v-model="newColumn.dataType"
        class="w-44 shrink-0"
        :options="types"
        aria-label="New column type"
      />
      <label class="flex items-center gap-1.5 text-xs text-fg-muted">
        <input
          v-model="newColumn.nullable"
          type="checkbox"
          aria-label="Nullable"
        />
        Null
      </label>
      <BaseButton size="sm" variant="primary" @click="addColumn"
        >Add</BaseButton
      >
      <IconButton :icon="X" size="sm" label="Cancel" @click="adding = false" />
    </div>

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
        @rename="(column, name) => rename(column, name)"
        @retype="(column, dataType) => retype(column, dataType)"
        @toggle-nullable="
          (column) =>
            changes.stage({
              kind: 'setNullable',
              name: column.name,
              nullable: !column.nullable,
              dataType: column.dataType,
            })
        "
        @drop="
          (column) => changes.stage({ kind: 'dropColumn', name: column.name })
        "
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

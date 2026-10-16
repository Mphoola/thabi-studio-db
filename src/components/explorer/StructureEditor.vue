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

const th =
  "h-7 border-b border-line bg-grid-header px-3 text-left text-xs font-semibold text-fg-muted";
const td = "h-7 border-b border-line px-3";
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
        {{ editing ? "Done" : "Edit structure" }}
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
        <span v-if="changes.dirty.value" class="text-xs text-fg-muted">
          {{ changes.edits.value.length }} staged
          {{ changes.edits.value.length === 1 ? "change" : "changes" }}
        </span>
        <BaseButton
          v-if="changes.dirty.value"
          size="sm"
          class="ml-auto"
          variant="primary"
          :loading="changes.previewing.value"
          @click="changes.review"
        >
          Review SQL
        </BaseButton>
        <BaseButton
          v-if="changes.dirty.value"
          size="sm"
          variant="ghost"
          @click="changes.discard"
        >
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

    <ul
      v-if="changes.dirty.value"
      class="flex flex-col gap-1 border-b border-line px-3 py-2"
    >
      <li
        v-for="one in changes.edits.value"
        :key="changes.describeEdit(one)"
        class="flex items-center gap-2 text-xs text-fg-muted"
      >
        <span>{{ changes.describeEdit(one) }}</span>
        <IconButton
          :icon="X"
          size="sm"
          label="Undo this change"
          @click="changes.undo(one)"
        />
      </li>
    </ul>

    <div class="selectable overflow-auto p-4">
      <table
        class="w-full border-collapse rounded-md border border-line text-sm"
      >
        <thead>
          <tr>
            <th :class="th" class="w-10">#</th>
            <th :class="th">Name</th>
            <th :class="th">Type</th>
            <th :class="th">Nullable</th>
            <th :class="th">Default</th>
            <th v-if="editing" :class="th" class="w-24">Change</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(column, index) in structure.columns" :key="column.name">
            <td :class="td" class="text-fg-subtle tabular-nums">
              {{ index + 1 }}
            </td>
            <td :class="td" class="font-medium">{{ column.name }}</td>
            <td :class="td" class="font-mono text-fg-muted">
              {{ column.dataType }}
            </td>
            <td :class="td" class="text-fg-muted">
              {{ column.nullable ? "Yes" : "No" }}
            </td>
            <td :class="td" class="max-w-64 truncate font-mono text-fg-muted">
              {{ column.default === null ? "—" : column.default }}
            </td>
            <td v-if="editing" :class="td">
              <div class="flex items-center gap-1">
                <BaseButton
                  size="sm"
                  variant="ghost"
                  @click="
                    changes.stage({
                      kind: 'setNullable',
                      name: column.name,
                      nullable: !column.nullable,
                      dataType: column.dataType,
                    })
                  "
                >
                  {{ column.nullable ? "Not null" : "Nullable" }}
                </BaseButton>
                <BaseButton
                  size="sm"
                  variant="ghost"
                  @click="
                    changes.stage({ kind: 'dropColumn', name: column.name })
                  "
                >
                  Drop
                </BaseButton>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <DdlReviewDialog
      v-model:open="reviewOpen"
      :title="`Change ${title}`"
      :preview="changes.preview.value"
      :applying="changes.applying.value"
      @close="reviewOpen = false"
      @apply="changes.run"
    />
  </div>
</template>

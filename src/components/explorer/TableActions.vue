<script setup>
import { computed, ref } from "vue";
import { Copy, CopyPlus, Scissors, Trash2, Pencil } from "lucide-vue-next";
import {
  applyTableAction,
  previewTableAction,
} from "../../lib/tauri/commands.js";
import { toUserDetail, toUserMessage } from "../../lib/errors.js";
import { confirm } from "../../composables/useConfirm.js";
import { useToasts } from "../../composables/useToasts.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import DropdownMenu from "../common/DropdownMenu.vue";
import DdlReviewDialog from "./DdlReviewDialog.vue";
import TextInput from "../common/TextInput.vue";

/**
 * The actions that work on a whole table.
 *
 * Everything here can lose data, so every one of them is confirmed by typing
 * the table's name — not "are you sure", which is a reflex rather than a
 * decision — and then shows the SQL it will run before running it. A copy
 * asked for twice is a nuisance; a table dropped is not undoable from here.
 */
const props = defineProps({
  tab: { type: Object, required: true },
  profileId: { type: String, required: true },
  database: { type: String, default: "" },
  schema: { type: String, default: "" },
  table: { type: String, required: true },
  disabled: { type: Boolean, default: false },
});
const emit = defineEmits(["changed", "renamed"]);

const toasts = useToasts();
const preview = ref(null);
const reviewOpen = ref(false);
const applying = ref(false);
const pending = ref(null);
const naming = ref(null);
const newName = ref("");

const items = [
  { id: "rename", label: "Rename…", icon: Pencil },
  { id: "duplicate", label: "Duplicate (structure only)…", icon: Copy },
  { id: "duplicate-data", label: "Duplicate with data…", icon: CopyPlus },
  { id: "truncate", label: "Empty the table…", icon: Scissors },
  { id: "drop", label: "Drop table…", icon: Trash2, danger: true },
];

/** What each action says, and whether it needs the name typed twice over. */
const COPY = {
  rename: {
    verb: "Rename",
    consequence: "Open tabs pointing at it will not follow the new name.",
  },
  duplicate: {
    verb: "Duplicate",
    consequence:
      "Makes a second table with the same columns and no rows. Nothing is copied or removed.",
  },
  "duplicate-data": {
    verb: "Duplicate with data",
    consequence: "Copies every row. This can take a while on a large table.",
  },
  truncate: {
    verb: "Empty",
    consequence:
      "Removes every row and keeps the structure. This cannot be undone.",
  },
  drop: {
    verb: "Drop",
    consequence:
      "Removes the table and everything in it. This cannot be undone.",
  },
};

async function choose(id) {
  if (id === "rename" || id === "duplicate" || id === "duplicate-data") {
    naming.value = id;
    newName.value = `${props.table}_copy`;
    return;
  }
  const copy = COPY[id];
  const ok = await confirm({
    title: `${copy.verb} “${props.table}”?`,
    message: copy.consequence,
    // Typed, because this is the one dialog where a reflex costs data.
    requireText: props.table,
    confirmLabel: copy.verb,
    destructive: true,
  });
  if (ok) await review(id);
}

async function review(id) {
  applying.value = true;
  try {
    pending.value = id;
    preview.value = await previewTableAction(
      props.profileId,
      props.tab.id,
      {
        database: props.database,
        schema: props.schema || null,
        name: props.table,
      },
      actionFor(id, naming.value === id ? newName.value.trim() : null),
    );
    reviewOpen.value = true;
  } catch (failure) {
    toasts.error(`Couldn't work out the SQL`, toUserMessage(failure));
  } finally {
    applying.value = false;
  }
}

function actionFor(id, name) {
  switch (id) {
    case "rename":
      return { rename: { to: name } };
    case "duplicate":
      return { duplicateStructure: { to: name } };
    case "duplicate-data":
      return { duplicateWithData: { to: name } };
    case "truncate":
      return { truncate: { restartIdentity: true } };
    default:
      return { drop: { ifExists: false } };
  }
}

async function run() {
  applying.value = true;
  try {
    await applyTableAction(
      props.profileId,
      props.tab.id,
      {
        database: props.database,
        schema: props.schema || null,
        name: props.table,
      },
      actionFor(
        pending.value,
        naming.value === pending.value ? newName.value.trim() : null,
      ),
    );
    const copy = COPY[pending.value];
    toasts.success(`${copy.verb} done`);
    reviewOpen.value = false;
    naming.value = null;
    pending.value = null;
    preview.value = null;
    emit("changed");
  } catch (failure) {
    // Same as the structure editor: a failed DDL batch may still have applied
    // part of it, so the table has to be re-read rather than left as it was.
    emit("changed");
    toasts.error(
      `Couldn't ${COPY[pending.value]?.verb.toLowerCase() ?? "run"} the table`,
      toUserDetail(failure),
    );
  } finally {
    applying.value = false;
  }
}

/** A copy needs a name, and the table's own name is not available. */
const canName = computed(
  () => Boolean(naming.value) && newName.value.trim() !== props.table,
);
</script>

<template>
  <DropdownMenu
    :items="items"
    label="Table actions"
    align="end"
    @select="choose"
  >
    <BaseButton size="sm" :disabled="disabled">Table actions</BaseButton>
  </DropdownMenu>

  <BaseDialog v-model:open="naming" size="sm" title="Name">
    <form class="flex flex-col gap-3" @submit.prevent="review(naming)">
      <p class="text-sm text-fg-muted">{{ COPY[naming]?.consequence }}</p>
      <TextInput v-model="newName" label="New name" autocomplete="off" />
    </form>
    <template #footer>
      <BaseButton size="sm" @click="naming = null">Cancel</BaseButton>
      <BaseButton
        size="sm"
        variant="primary"
        :disabled="!canName"
        :loading="applying"
        @click="review(naming)"
      >
        Review SQL
      </BaseButton>
    </template>
  </BaseDialog>

  <DdlReviewDialog
    v-model:open="reviewOpen"
    :title="`${COPY[pending]?.verb ?? 'Change'} ${table}`"
    :preview="preview"
    :applying="applying"
    @close="reviewOpen = false"
    @apply="run"
  />
</template>

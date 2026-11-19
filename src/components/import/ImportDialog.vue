<script setup>
import { computed, ref } from "vue";
import { confirm } from "../../composables/useConfirm.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useImport } from "../../composables/useImport.js";
import { typesFor } from "../../lib/createTable.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import ImportColumnsStep from "./ImportColumnsStep.vue";
import ImportFileStep from "./ImportFileStep.vue";
import ImportRunStep from "./ImportRunStep.vue";

/**
 * Imports a CSV, TSV or JSON file into a table, or into a new one: choose
 * and preview the file, match its columns, then run it with progress. Rows
 * go in batches inside one transaction, so a stop leaves nothing behind.
 */
const open = defineModel("open", { type: Boolean, default: false });
const props = defineProps({
  /** { profileId, database, schema, table? } */
  target: { type: Object, required: true },
});
const emit = defineEmits(["done"]);

const state = useImport(props.target);
const step = ref("file");
const connections = useConnectionProfiles();
const profile = computed(() =>
  connections.profiles.value.find((p) => p.id === props.target.profileId),
);
const into = computed(
  () => props.target.table || state.newTable.name.trim() || "a new table",
);
const canNext = computed(() => {
  if (step.value === "file") return Boolean(state.preview.value);
  if (step.value === "columns")
    return (
      state.mappings.value.length > 0 &&
      (Boolean(props.target.table) || Boolean(state.newTable.name.trim()))
    );
  return false;
});

async function start() {
  // Writes go through the connection's safe mode like any other change.
  if (
    ["confirm-writes", "confirm-everything"].includes(profile.value?.safeMode)
  ) {
    const ok = await confirm({
      title: `Import into “${into.value}”?`,
      message: `On “${profile.value.name}”. Rows go in one transaction; a stop keeps nothing.`,
      confirmLabel: "Import",
      requireText:
        profile.value.environment === "production" ? into.value : null,
    });
    if (!ok) return;
  }
  step.value = "run";
  await state.run();
  emit("done");
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    size="lg"
    :title="`Import into ${target.table ? `“${target.table}”` : 'a New Table'}`"
  >
    <ImportFileStep v-if="step === 'file'" :state="state" />
    <ImportColumnsStep
      v-else-if="step === 'columns'"
      :state="state"
      :existing="Boolean(target.table)"
      :types="typesFor(profile?.driver)"
    />
    <ImportRunStep v-else :state="state" />
    <template #footer>
      <template v-if="step !== 'run'">
        <BaseButton v-if="step === 'columns'" @click="step = 'file'"
          >Back</BaseButton
        >
        <BaseButton @click="open = false">Cancel</BaseButton>
        <BaseButton
          v-if="step === 'file'"
          variant="primary"
          :disabled="!canNext"
          @click="step = 'columns'"
          >Next</BaseButton
        >
        <BaseButton v-else variant="primary" :disabled="!canNext" @click="start"
          >Import</BaseButton
        >
      </template>
      <template v-else>
        <BaseButton v-if="state.running.value" @click="state.cancel()"
          >Stop</BaseButton
        >
        <BaseButton v-else variant="primary" @click="open = false"
          >Done</BaseButton
        >
      </template>
    </template>
  </BaseDialog>
</template>

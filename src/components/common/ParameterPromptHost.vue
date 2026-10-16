<script setup>
import { computed, ref, watch } from "vue";
import { useParameterPrompt } from "../../composables/useParameterPrompt.js";
import BaseButton from "./BaseButton.vue";
import BaseDialog from "./BaseDialog.vue";
import SelectInput from "./SelectInput.vue";
import TextInput from "./TextInput.vue";

/** Values for a statement's placeholders, asked for just before it runs. */
const { request, settle } = useParameterPrompt();
const answers = ref([]);
const types = [
  { value: "text", label: "Text" },
  { value: "number", label: "Number" },
  { value: "null", label: "NULL" },
];

const open = computed({
  get: () => request.value !== null,
  set: (isOpen) => !isOpen && settle(null),
});

watch(request, (value) => {
  answers.value = (value?.labels ?? []).map((_, i) => ({
    type: value.defaults?.[i]?.type ?? "text",
    value: value.defaults?.[i]?.value ?? "",
  }));
});

const errors = computed(() =>
  answers.value.map((a) =>
    a.type === "number" && !/^\s*-?\d+(\.\d+)?\s*$/.test(a.value)
      ? "Enter a number, like 42 or 3.14."
      : "",
  ),
);

function run() {
  if (errors.value.some(Boolean)) return;
  settle(answers.value.map((a) => ({ ...a })));
}
</script>

<template>
  <BaseDialog
    v-if="request"
    v-model:open="open"
    size="md"
    title="Query Parameters"
    description="Values are sent to the server separately from the SQL, so quotes and semicolons in them are just text."
  >
    <form
      id="parameters-form"
      class="flex max-h-[55vh] flex-col gap-3 overflow-y-auto"
      @submit.prevent="run"
    >
      <div
        v-for="(label, index) in request.labels"
        :key="label"
        class="grid grid-cols-[1fr_110px] items-start gap-2"
      >
        <TextInput
          v-model="answers[index].value"
          :label="label"
          :disabled="answers[index].type === 'null'"
          :error="errors[index]"
          mono
        />
        <SelectInput
          v-model="answers[index].type"
          label="Type"
          :options="types"
        />
      </div>
    </form>
    <template #footer>
      <BaseButton @click="settle(null)">Cancel</BaseButton>
      <BaseButton variant="primary" type="submit" form="parameters-form">
        Run
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<script setup>
import { computed, ref, watch } from "vue";
import { useAppLock } from "../../../composables/useAppLock.js";
import { useToasts } from "../../../composables/useToasts.js";
import {
  QUESTION_COUNT,
  SECURITY_QUESTIONS,
} from "../../../lib/appPassword.js";
import { toUserMessage } from "../../../lib/errors.js";
import BaseButton from "../../common/BaseButton.vue";
import BaseDialog from "../../common/BaseDialog.vue";
import InlineAlert from "../../common/InlineAlert.vue";
import PasswordInput from "../../common/PasswordInput.vue";
import SelectInput from "../../common/SelectInput.vue";
import TextInput from "../../common/TextInput.vue";

/**
 * Sets up (or removes) three security questions. All three must be answered
 * to set a new password from the lock screen. Answers ignore case and extra
 * spaces, and are stored only as hashes.
 */
const open = defineModel("open", { type: Boolean, default: false });
const props = defineProps({
  /** Remove the saved questions instead of setting new ones. */
  remove: { type: Boolean, default: false },
});
const emit = defineEmits(["changed"]);

const OWN = "__own__";
const lock = useAppLock();
const toasts = useToasts();
const current = ref("");
const rows = ref([]);
const errors = ref({});
const failure = ref("");
const busy = ref(false);

watch(open, (isOpen) => {
  if (!isOpen) return;
  current.value = "";
  errors.value = {};
  failure.value = "";
  rows.value = Array.from({ length: QUESTION_COUNT }, (_, index) => ({
    choice: SECURITY_QUESTIONS[index],
    own: "",
    answer: "",
  }));
});

const choices = [
  ...SECURITY_QUESTIONS.map((question) => ({
    value: question,
    label: question,
  })),
  { value: OWN, label: "Write my own question…" },
];
const questionOf = (row) => (row.choice === OWN ? row.own : row.choice);
const canSubmit = computed(
  () =>
    Boolean(current.value) &&
    (props.remove ||
      rows.value.every((row) => questionOf(row).trim() && row.answer.trim())),
);

async function submit() {
  if (!canSubmit.value || busy.value) return;
  busy.value = true;
  errors.value = {};
  failure.value = "";
  try {
    if (props.remove) {
      await lock.removeQuestions(current.value);
      toasts.info("Security questions removed");
    } else {
      await lock.setQuestions(
        current.value,
        rows.value.map((row) => ({
          question: questionOf(row),
          answer: row.answer,
        })),
      );
      toasts.success("Security questions saved");
    }
    emit("changed");
    open.value = false;
  } catch (problem) {
    if (problem?.fields) errors.value = problem.fields;
    else failure.value = toUserMessage(problem);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    :title="remove ? 'Remove Security Questions' : 'Security Questions'"
    :size="remove ? 'sm' : 'md'"
  >
    <form
      id="security-questions-form"
      class="flex flex-col gap-4"
      @submit.prevent="submit"
    >
      <p v-if="!remove" class="text-sm text-fg-muted">
        If you forget the app password, answering all three lets you set a new
        one. Pick answers only you would know, and that you'll write the same
        way later. Case and extra spaces don't matter.
      </p>
      <fieldset
        v-for="(row, index) in remove ? [] : rows"
        :key="index"
        class="flex flex-col gap-2"
      >
        <legend class="mb-1 text-sm font-medium">
          Question {{ index + 1 }}
        </legend>
        <SelectInput
          v-model="row.choice"
          :options="choices"
          :aria-label="`Question ${index + 1}`"
          :error="row.choice === OWN ? '' : errors[`question${index}`]"
        />
        <TextInput
          v-if="row.choice === OWN"
          v-model="row.own"
          placeholder="Your question"
          :aria-label="`Your question ${index + 1}`"
          :error="errors[`question${index}`]"
        />
        <TextInput
          v-model="row.answer"
          placeholder="Answer"
          :aria-label="`Answer ${index + 1}`"
          :error="errors[`answer${index}`]"
        />
      </fieldset>
      <PasswordInput
        v-model="current"
        label="Current app password"
        :error="errors.current"
      />
      <InlineAlert
        v-if="failure"
        tone="danger"
        title="Couldn't save"
        :message="failure"
      />
    </form>
    <template #footer>
      <BaseButton @click="open = false">Cancel</BaseButton>
      <BaseButton
        type="submit"
        form="security-questions-form"
        :variant="remove ? 'danger' : 'primary'"
        :disabled="!canSubmit"
        :loading="busy"
        >{{ remove ? "Remove" : "Save Questions" }}</BaseButton
      >
    </template>
  </BaseDialog>
</template>

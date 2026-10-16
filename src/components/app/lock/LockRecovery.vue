<script setup>
import { computed, onMounted, ref } from "vue";
import { KeyRound } from "lucide-vue-next";
import { useAppLock } from "../../../composables/useAppLock.js";
import { isStrongPassword } from "../../../lib/appPassword.js";
import { toUserMessage } from "../../../lib/errors.js";
import BaseButton from "../../common/BaseButton.vue";
import InlineAlert from "../../common/InlineAlert.vue";
import SegmentedControl from "../../common/SegmentedControl.vue";
import TextInput from "../../common/TextInput.vue";
import LockReset from "../LockReset.vue";
import NewPasswordFields from "./NewPasswordFields.vue";

/**
 * Forgotten password: a recovery code or the security questions set a new
 * password. Without either, the reset (which deletes saved passwords) is the
 * last way in.
 */
const emit = defineEmits(["back"]);
const lock = useAppLock();
const options = ref(null);
const loadError = ref("");
const method = ref("code");
const code = ref("");
const answers = ref([]);
const password = ref("");
const repeat = ref("");
const errors = ref({});
const failure = ref("");
const busy = ref(false);
const resetting = ref(false);

onMounted(async () => {
  try {
    options.value = await lock.recoveryOptions();
    answers.value = options.value.questions.map(() => "");
    if (!options.value.codesLeft && options.value.questions.length)
      method.value = "questions";
  } catch (error) {
    loadError.value = toUserMessage(error);
  }
});

const methods = computed(() => [
  ...(options.value?.codesLeft
    ? [{ value: "code", label: "Recovery code" }]
    : []),
  ...(options.value?.questions.length
    ? [{ value: "questions", label: "Security questions" }]
    : []),
]);
const canRecover = computed(
  () => options.value?.codesLeft || options.value?.questions.length,
);
const proofGiven = computed(() =>
  method.value === "code"
    ? code.value.trim().length > 0
    : answers.value.every((answer) => answer.trim()),
);
const canSubmit = computed(
  () =>
    proofGiven.value &&
    isStrongPassword(password.value) &&
    repeat.value === password.value,
);

async function submit() {
  if (!canSubmit.value || busy.value) return;
  busy.value = true;
  errors.value = {};
  failure.value = "";
  try {
    if (method.value === "code")
      await lock.recoverWithCode(code.value, password.value);
    else await lock.recoverWithAnswers(answers.value, password.value);
  } catch (problem) {
    if (problem?.fields) errors.value = problem.fields;
    else failure.value = toUserMessage(problem);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <LockReset v-if="resetting" @back="resetting = false" />
  <form
    v-else
    class="flex w-full max-w-sm flex-col gap-4"
    @submit.prevent="submit"
  >
    <h1 id="lock-title" class="flex items-center gap-1.5 text-lg font-semibold">
      <KeyRound :size="16" :stroke-width="1.75" aria-hidden="true" />
      Set a new app password
    </h1>
    <InlineAlert
      v-if="loadError"
      tone="danger"
      title="Recovery options couldn't be read"
      :message="loadError"
    />
    <p v-else-if="options && !canRecover" class="text-sm text-fg-muted">
      There are no recovery codes or security questions left. The only way in is
      to reset the password.
    </p>
    <template v-else-if="options">
      <SegmentedControl
        v-if="methods.length > 1"
        v-model="method"
        :options="methods"
        label="Recover with"
      />
      <TextInput
        v-if="method === 'code'"
        v-model="code"
        mono
        placeholder="xxxxx-xxxxx"
        aria-label="Recovery code"
        :hint="`${options.codesLeft} unused. The code is used up once it works.`"
        :error="errors.code"
      />
      <template v-else>
        <TextInput
          v-for="(question, index) in options.questions"
          :key="question"
          v-model="answers[index]"
          :label="question"
        />
        <p v-if="errors.answers" class="text-sm text-danger-text" role="alert">
          {{ errors.answers }}
        </p>
      </template>
      <NewPasswordFields
        v-model="password"
        v-model:repeat="repeat"
        :error="errors.password"
      />
      <p v-if="failure" class="text-sm text-danger-text" role="alert">
        {{ failure }}
      </p>
    </template>

    <div class="flex items-center justify-between gap-2">
      <button
        type="button"
        class="text-sm text-fg-subtle hover:text-fg-muted hover:underline"
        @click="resetting = true"
      >
        Reset instead…
      </button>
      <div class="flex gap-2">
        <BaseButton :disabled="busy" @click="emit('back')">Back</BaseButton>
        <BaseButton
          v-if="canRecover"
          type="submit"
          variant="primary"
          :disabled="!canSubmit"
          :loading="busy"
          >Set Password</BaseButton
        >
      </div>
    </div>
  </form>
</template>

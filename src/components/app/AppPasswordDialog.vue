<script setup>
import { computed, ref, watch } from "vue";
import { useAppLock } from "../../composables/useAppLock.js";
import { useToasts } from "../../composables/useToasts.js";
import { isStrongPassword } from "../../lib/appPassword.js";
import { toUserMessage } from "../../lib/errors.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import InlineAlert from "../common/InlineAlert.vue";
import PasswordInput from "../common/PasswordInput.vue";
import NewPasswordFields from "./lock/NewPasswordFields.vue";
import RecoveryCodes from "./lock/RecoveryCodes.vue";

/**
 * Sets, changes or turns off the app password, or makes new recovery codes.
 * Setting it ends on the new recovery codes, which are shown only once.
 */
const open = defineModel("open", { type: Boolean, default: false });
const props = defineProps({
  /** "set", "change", "remove" or "codes". */
  mode: { type: String, default: "set" },
});
const emit = defineEmits(["changed"]);

const lock = useAppLock();
const toasts = useToasts();
const current = ref("");
const password = ref("");
const repeat = ref("");
const errors = ref({});
const failure = ref("");
const busy = ref(false);
const codes = ref(null);
const codesSaved = ref(false);

watch(open, (isOpen) => {
  if (!isOpen) return;
  current.value = password.value = repeat.value = "";
  errors.value = {};
  failure.value = "";
  codes.value = null;
  codesSaved.value = false;
});

const copy = computed(
  () =>
    ({
      set: { title: "Set App Password", action: "Set Password" },
      change: { title: "Change App Password", action: "Change Password" },
      remove: { title: "Turn Off App Password", action: "Turn Off" },
      codes: { title: "New Recovery Codes", action: "Make New Codes" },
    })[props.mode],
);
const needsCurrent = computed(() => props.mode !== "set");
const needsNew = computed(() => ["set", "change"].includes(props.mode));
const canSubmit = computed(() =>
  needsNew.value
    ? isStrongPassword(password.value) &&
      repeat.value === password.value &&
      (!needsCurrent.value || current.value)
    : Boolean(current.value),
);

async function run() {
  if (props.mode === "remove") {
    await lock.removePassword(current.value);
    toasts.info("App password turned off");
    return null;
  }
  if (props.mode === "codes") return lock.regenerateCodes(current.value);
  const fresh = await lock.setPassword(current.value, password.value);
  if (props.mode === "change") toasts.success("App password changed");
  return fresh;
}

async function submit() {
  if (!canSubmit.value || busy.value) return;
  busy.value = true;
  errors.value = {};
  failure.value = "";
  try {
    const fresh = await run();
    emit("changed");
    if (fresh) codes.value = fresh;
    else open.value = false;
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
    :title="codes ? 'Save Your Recovery Codes' : copy.title"
    :size="codes ? 'md' : 'sm'"
  >
    <RecoveryCodes v-if="codes" v-model:saved="codesSaved" :codes="codes" />
    <form
      v-else
      id="app-password-form"
      class="flex flex-col gap-3"
      @submit.prevent="submit"
    >
      <p v-if="mode === 'set'" class="text-sm text-fg-muted">
        Thabi Studio will ask for it when it opens and after it has been idle.
        Next you'll get recovery codes, in case you forget it.
      </p>
      <p v-else-if="mode === 'codes'" class="text-sm text-fg-muted">
        New codes replace all the old ones, used or not.
      </p>
      <PasswordInput
        v-if="needsCurrent"
        v-model="current"
        label="Current password"
        :error="errors.current"
      />
      <NewPasswordFields
        v-if="needsNew"
        v-model="password"
        v-model:repeat="repeat"
        :label="mode === 'set' ? 'Password' : 'New password'"
        :error="errors.password"
      />
      <InlineAlert
        v-if="failure"
        tone="danger"
        title="Couldn't save"
        :message="failure"
      />
    </form>
    <template #footer>
      <BaseButton
        v-if="codes"
        variant="primary"
        :disabled="!codesSaved"
        @click="open = false"
        >Done</BaseButton
      >
      <template v-else>
        <BaseButton @click="open = false">Cancel</BaseButton>
        <BaseButton
          type="submit"
          form="app-password-form"
          :variant="mode === 'remove' ? 'danger' : 'primary'"
          :disabled="!canSubmit"
          :loading="busy"
          >{{ copy.action }}</BaseButton
        >
      </template>
    </template>
  </BaseDialog>
</template>

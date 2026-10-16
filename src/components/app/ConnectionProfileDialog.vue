<script setup>
import { words } from "../../lib/platform.js";
import { nextTick, ref } from "vue";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useConnectionActions } from "../../composables/useConnectionActions.js";
import { useConnectionForm } from "./connection/useConnectionForm.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserMessage } from "../../lib/errors.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import ConnectionTestAction from "./ConnectionTestAction.vue";
import ConnectionTestResult from "./ConnectionTestResult.vue";
import InlineAlert from "../common/InlineAlert.vue";
import AdvancedCard from "./connection/AdvancedCard.vue";
import DatabaseCard from "./connection/DatabaseCard.vue";
import IdentityCard from "./connection/IdentityCard.vue";
import SshCard from "./connection/SshCard.vue";

const store = useConnectionProfiles();
const toasts = useToasts();
const fields = ref({});
const saving = ref(false);
const testResult = ref(null);
const saveError = ref("");
const formEl = ref(null);
const advanced = ref(null);
const actions = useConnectionActions();
const {
  form,
  open,
  editingProfile,
  tlsTouched,
  safeModeTouched,
  buildInput,
  fillFromUrl,
} = useConnectionForm({ fields, testResult });

function onTestResult(result) {
  if (result && !result.ok && result.error?.kind === "validation") {
    fields.value = result.error.fields ?? {};
    return;
  }
  testResult.value = result;
}

/**
 * Saves the connection; with `connect`, opens it straight away (TablePlus's
 * Connect button). Field errors show next to their fields and scroll into
 * view; anything else is shown in the dialog, not behind it.
 */
async function save(connect = false) {
  saveError.value = "";
  const input = buildInput();
  if (!input) return revealErrors();
  saving.value = true;
  try {
    const { warning, profile } = await store.save(input);
    if (warning)
      toasts.error(
        `Saved, but ${words.credentialStore} needs attention`,
        warning,
      );
    if (connect) await actions.connect(profile);
  } catch (error) {
    if (error?.kind === "validation" && error.fields) {
      fields.value = error.fields;
      revealErrors();
    } else saveError.value = toUserMessage(error);
  } finally {
    saving.value = false;
  }
}

/** Opens the section holding the first error and scrolls it into view. */
async function revealErrors() {
  advanced.value?.reveal();
  await nextTick();
  formEl.value
    ?.querySelector('[role="alert"], [aria-invalid="true"]')
    ?.scrollIntoView({ block: "center", behavior: "smooth" });
}
</script>

<template>
  <BaseDialog
    v-model:open="open"
    size="lg"
    :title="editingProfile ? 'Edit Connection' : 'New Connection'"
  >
    <form
      id="connection-profile-form"
      ref="formEl"
      class="flex max-h-[min(70vh,680px)] flex-col gap-4 overflow-y-auto pr-1"
      @submit.prevent="save(false)"
    >
      <IdentityCard
        v-model:form="form"
        :fields="fields"
        @safe-mode-changed="safeModeTouched = true"
      />
      <DatabaseCard
        v-model:form="form"
        :fields="fields"
        :has-saved-password="!!editingProfile?.hasDatabasePassword"
        @tls-touched="tlsTouched = true"
      />
      <SshCard
        v-model:form="form"
        :fields="fields"
        :has-saved-password="!!editingProfile?.hasSshPassword"
        :has-saved-passphrase="!!editingProfile?.hasSshKeyPassphrase"
      />
      <AdvancedCard
        ref="advanced"
        v-model:form="form"
        :fields="fields"
        @fill-url="fillFromUrl"
      />
      <ConnectionTestResult v-if="testResult" :result="testResult" />
      <InlineAlert
        v-if="saveError"
        tone="danger"
        title="Couldn’t save the connection"
        :message="saveError"
      />
    </form>
    <template #footer>
      <ConnectionTestAction
        class="mr-auto"
        :build-input="buildInput"
        :profile-id="editingProfile?.id ?? null"
        :disabled="!store.nativeAvailable"
        @result="onTestResult"
      />
      <BaseButton @click="open = false">Cancel</BaseButton>
      <BaseButton
        type="submit"
        form="connection-profile-form"
        :loading="saving"
      >
        {{ editingProfile ? "Save Changes" : "Save" }}
      </BaseButton>
      <BaseButton variant="primary" :loading="saving" @click="save(true)">
        Connect
      </BaseButton>
    </template>
  </BaseDialog>
</template>

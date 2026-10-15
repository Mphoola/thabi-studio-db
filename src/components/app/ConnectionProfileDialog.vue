<script setup>
import { words } from "../../lib/platform.js";
import { computed, nextTick, reactive, ref, watch } from "vue";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useDialogs } from "../../composables/useDialogs.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserMessage } from "../../lib/errors.js";
import { ENVIRONMENTS } from "../../lib/environments.js";
import { COLOURS } from "../../lib/connectionGroups.js";
import { defaultSafeMode } from "../../lib/sqlSafety.js";
import { suggestedTlsMode } from "../../lib/tls.js";
import {
  defaultPortFor,
  validateConnectionProfile,
} from "../../lib/connectionValidation.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import SelectInput from "../common/SelectInput.vue";
import TextInput from "../common/TextInput.vue";
import ConnectionTestAction from "./ConnectionTestAction.vue";
import ConnectionTestResult from "./ConnectionTestResult.vue";
import DatabaseFields from "./DatabaseFields.vue";
import SafetyFields from "./SafetyFields.vue";
import SshProfileFields from "./SshProfileFields.vue";
import ConnectionUrlField from "../connections/ConnectionUrlField.vue";

const store = useConnectionProfiles();
const dialogs = useDialogs();
const toasts = useToasts();
const fields = ref({});
const saving = ref(false);
const testResult = ref(null);
// Until the user picks a TLS setting, it follows the host (see lib/tls.js).
const tlsTouched = ref(false);
// The safe mode follows the environment until the user picks one (Plan §7b:
// Production starts on "Confirm Writes").
const safeModeTouched = ref(false);

const defaults = () => ({
  name: "",
  environment: "local",
  driver: "postgres",
  host: "localhost",
  port: "5432",
  databaseName: "",
  username: "",
  databasePassword: "",
  clearDatabasePassword: false,
  tlsMode: suggestedTlsMode("localhost"),
  tlsCaPath: "",
  sshEnabled: false,
  sshHost: "",
  sshPort: "22",
  sshUsername: "",
  sshPassword: "",
  clearSshPassword: false,
  sshAuthMode: "password",
  sshKeyPath: "",
  sshKeyPassphrase: "",
  clearSshKeyPassphrase: false,
  safeMode: defaultSafeMode("local"),
  groupName: "",
  colour: "",
  askDatabasePassword: false,
  askSshPassword: false,
  askSshKeyPassphrase: false,
  sshKeepaliveSeconds: "0",
});
const form = reactive(defaults());
const open = computed({
  get: () => dialogs.open.value === "connection",
  set: (value) => (value ? dialogs.show("connection") : dialogs.close()),
});
const editingProfile = computed(
  () =>
    store.profiles.value.find(
      (profile) => profile.id === store.editingId.value,
    ) ?? null,
);
const environmentOptions = ENVIRONMENTS.map(({ value, label }) => ({
  value,
  label,
}));

function resetForm() {
  Object.assign(form, defaults());
  fields.value = {};
  testResult.value = null;
  tlsTouched.value = false;
  safeModeTouched.value = false;
  const profile = editingProfile.value;
  if (!profile) return;
  tlsTouched.value = true;
  safeModeTouched.value = true;
  Object.assign(form, {
    name: profile.name,
    environment: profile.environment,
    driver: profile.driver,
    host: profile.host,
    port: String(profile.port),
    databaseName: profile.databaseName,
    username: profile.username,
    tlsMode: profile.tlsMode,
    tlsCaPath: profile.tlsCaPath,
    sshEnabled: profile.sshEnabled,
    sshHost: profile.sshHost,
    sshPort: String(profile.sshPort),
    sshUsername: profile.sshUsername,
    sshAuthMode: profile.sshAuthMode,
    sshKeyPath: profile.sshKeyPath,
    safeMode: profile.safeMode ?? defaultSafeMode(profile.environment),
    groupName: profile.groupName ?? "",
    colour: profile.colour ?? "",
    askDatabasePassword: !!profile.askDatabasePassword,
    askSshPassword: !!profile.askSshPassword,
    askSshKeyPassphrase: !!profile.askSshKeyPassphrase,
    sshKeepaliveSeconds: String(profile.sshKeepaliveSeconds ?? 0),
  });
}

watch(
  () => [open.value, store.editingId.value],
  ([isOpen]) => isOpen && resetForm(),
);
watch(
  () => form.driver,
  (next, previous) => {
    if (form.port === defaultPortFor(previous))
      form.port = defaultPortFor(next);
  },
);
watch(
  () => form.host,
  (host) => {
    if (!tlsTouched.value) form.tlsMode = suggestedTlsMode(host);
  },
);
watch(
  () => form.environment,
  (environment, previous) => {
    if (safeModeTouched.value && form.safeMode !== defaultSafeMode(previous))
      return;
    form.safeMode = defaultSafeMode(environment);
  },
);
watch(
  () => form.sshEnabled,
  (enabled) => {
    if (!enabled) form.sshPassword = "";
    // Through a tunnel the database is usually on the SSH server itself.
    else if (!form.host.trim()) form.host = "localhost";
  },
);
// A test result describes the values it ran with; clear it when they change.
watch(
  () => JSON.stringify(form),
  () => (testResult.value = null),
);

/** The request body for Rust, or null (with field errors) when invalid. */
function buildInput() {
  fields.value = validateConnectionProfile(form);
  if (Object.keys(fields.value).length) return null;
  const ssh = form.sshEnabled;
  return {
    ...form,
    // A secret that is asked for each time is never sent to be saved.
    databasePassword: form.askDatabasePassword ? "" : form.databasePassword,
    sshPassword:
      ssh && form.sshAuthMode === "password" && !form.askSshPassword
        ? form.sshPassword
        : "",
    sshKeyPassphrase:
      ssh && form.sshAuthMode === "key" && !form.askSshKeyPassphrase
        ? form.sshKeyPassphrase
        : "",
  };
}

async function fillFromUrl(parsed) {
  Object.assign(form, parsed);
  if (parsed.tlsMode) tlsTouched.value = true;
  if (!form.name.trim()) form.name = parsed.databaseName || parsed.host;
  fields.value = {};
  // The driver watcher may reset the port to a default; keep the URL's.
  await nextTick();
  form.port = parsed.port;
}

function onTestResult(result) {
  if (result && !result.ok && result.error?.kind === "validation") {
    fields.value = result.error.fields ?? {};
    return;
  }
  testResult.value = result;
}

async function save() {
  const input = buildInput();
  if (!input) return;
  saving.value = true;
  try {
    const warning = await store.save(input);
    if (warning)
      toasts.error(
        `Saved, but ${words.credentialStore} needs attention`,
        warning,
      );
  } catch (error) {
    if (error?.kind === "validation" && error.fields)
      fields.value = error.fields;
    else toasts.error("Couldn't save the connection", toUserMessage(error));
  } finally {
    saving.value = false;
  }
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
      class="flex max-h-[min(62vh,600px)] flex-col gap-4 overflow-y-auto"
      @submit.prevent="save"
    >
      <ConnectionUrlField @fill="fillFromUrl" />
      <div class="grid grid-cols-2 gap-3">
        <TextInput
          v-model="form.name"
          label="Name"
          placeholder="Orders (local)"
          :error="fields.name"
        />
        <SelectInput
          v-model="form.environment"
          label="Environment"
          :options="environmentOptions"
          :error="fields.environment"
        />
      </div>
      <div class="grid grid-cols-2 gap-3">
        <TextInput
          v-model="form.groupName"
          label="Group"
          placeholder="Work"
          :error="fields.groupName"
          hint="Leave empty to keep it with its environment."
        />
        <SelectInput
          v-model="form.colour"
          label="Colour"
          :options="COLOURS"
          :error="fields.colour"
          hint="Overrides the environment colour."
        />
      </div>
      <SafetyFields
        v-model:form="form"
        :fields="fields"
        @changed="safeModeTouched = true"
      />
      <DatabaseFields
        v-model:form="form"
        :fields="fields"
        :has-saved-password="!!editingProfile?.hasDatabasePassword"
        :via-ssh="form.sshEnabled"
        @tls-touched="tlsTouched = true"
      />
      <SshProfileFields
        v-model:form="form"
        :fields="fields"
        :has-saved-password="!!editingProfile?.hasSshPassword"
        :has-saved-passphrase="!!editingProfile?.hasSshKeyPassphrase"
      />
      <ConnectionTestResult v-if="testResult" :result="testResult" />
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
        variant="primary"
        type="submit"
        form="connection-profile-form"
        :loading="saving"
      >
        {{ editingProfile ? "Save Changes" : "Save Connection" }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

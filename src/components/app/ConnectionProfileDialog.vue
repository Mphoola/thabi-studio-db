<script setup>
import { computed, reactive, ref, watch } from "vue";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useDialogs } from "../../composables/useDialogs.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserMessage } from "../../lib/errors.js";
import { ENVIRONMENTS } from "../../lib/environments.js";
import {
  defaultPortFor,
  validateConnectionProfile,
} from "../../lib/connectionValidation.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import CheckboxInput from "../common/CheckboxInput.vue";
import PasswordInput from "../common/PasswordInput.vue";
import SelectInput from "../common/SelectInput.vue";
import TextInput from "../common/TextInput.vue";
import ConnectionTestAction from "./ConnectionTestAction.vue";
import SshProfileFields from "./SshProfileFields.vue";

const store = useConnectionProfiles();
const dialogs = useDialogs();
const toasts = useToasts();
const fields = ref({});
const saving = ref(false);
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
  sshEnabled: false,
  sshHost: "",
  sshPort: "22",
  sshUsername: "",
  sshPassword: "",
  clearSshPassword: false,
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
const title = computed(() =>
  editingProfile.value ? "Edit Connection" : "New Connection",
);

const drivers = [
  { value: "postgres", label: "PostgreSQL" },
  { value: "mysql", label: "MySQL" },
  { value: "sqlserver", label: "SQL Server" },
];
const environmentOptions = ENVIRONMENTS.map(({ value, label }) => ({
  value,
  label,
}));

function resetForm() {
  Object.assign(form, defaults());
  fields.value = {};
  if (!editingProfile.value) return;
  Object.assign(form, {
    name: editingProfile.value.name,
    environment: editingProfile.value.environment,
    driver: editingProfile.value.driver,
    host: editingProfile.value.host,
    port: String(editingProfile.value.port),
    databaseName: editingProfile.value.databaseName,
    username: editingProfile.value.username,
    sshEnabled: editingProfile.value.sshEnabled,
    sshHost: editingProfile.value.sshHost,
    sshPort: String(editingProfile.value.sshPort),
    sshUsername: editingProfile.value.sshUsername,
  });
}

watch(
  () => [open.value, store.editingId.value],
  ([isOpen]) => {
    if (isOpen) resetForm();
  },
);

watch(
  () => form.driver,
  (next, previous) => {
    if (form.port === defaultPortFor(previous))
      form.port = defaultPortFor(next);
  },
);

watch(
  () => form.sshEnabled,
  (enabled) => {
    if (!enabled) form.sshPassword = "";
  },
);

async function save() {
  fields.value = validateConnectionProfile(form);
  if (Object.keys(fields.value).length) return;

  saving.value = true;
  try {
    const warning = await store.save({
      name: form.name,
      environment: form.environment,
      driver: form.driver,
      host: form.host,
      port: form.port,
      databaseName: form.databaseName,
      username: form.username,
      databasePassword: form.databasePassword,
      clearDatabasePassword: form.clearDatabasePassword,
      sshEnabled: form.sshEnabled,
      sshHost: form.sshHost,
      sshPort: form.sshPort,
      sshUsername: form.sshUsername,
      sshPassword: form.sshEnabled ? form.sshPassword : "",
      clearSshPassword: form.clearSshPassword,
    });
    if (warning) toasts.error("Profile saved with a Keychain warning", warning);
    else toasts.success("Connection profile saved");
  } catch (error) {
    if (error?.kind === "validation" && error.fields) {
      fields.value = error.fields;
    } else {
      toasts.error("Couldn't save connection profile", toUserMessage(error));
    }
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <BaseDialog v-model:open="open" size="lg" :title="title">
    <form
      id="connection-profile-form"
      class="flex max-h-[min(62vh,560px)] flex-col gap-4 overflow-y-auto"
      @submit.prevent="save"
    >
      <div class="grid grid-cols-2 gap-3">
        <TextInput
          v-model="form.name"
          label="Connection name"
          placeholder="Production"
          :error="fields.name"
        />
        <SelectInput
          v-model="form.environment"
          label="Environment"
          :options="environmentOptions"
          :error="fields.environment"
        />
      </div>

      <section class="flex flex-col gap-3">
        <h3 class="text-sm font-semibold text-fg">Database</h3>
        <div class="grid grid-cols-2 gap-3">
          <SelectInput
            v-model="form.driver"
            label="Database type"
            :options="drivers"
            :error="fields.driver"
          />
          <TextInput
            v-model="form.host"
            label="Host"
            placeholder="localhost"
            :error="fields.host"
            autocomplete="url"
          />
          <TextInput
            v-model="form.port"
            label="Port"
            input-mode="numeric"
            :error="fields.port"
            mono
          />
          <TextInput
            v-model="form.databaseName"
            label="Database"
            hint="Optional. You can choose one after connecting."
            :error="fields.databaseName"
          />
          <TextInput
            v-model="form.username"
            label="User name"
            :error="fields.username"
            autocomplete="username"
          />
          <PasswordInput
            v-model="form.databasePassword"
            label="Password"
            :hint="
              editingProfile?.hasDatabasePassword
                ? 'Saved in Keychain. Leave blank to keep it.'
                : 'Optional. Stored in Keychain, never in the profile database.'
            "
            :error="fields.databasePassword"
          />
        </div>
        <CheckboxInput
          v-if="editingProfile?.hasDatabasePassword"
          v-model="form.clearDatabasePassword"
          label="Remove saved database password"
        />
        <p class="text-sm text-fg-subtle">
          Username and password authentication only. TLS certificate
          verification is on by default; more options arrive with database
          drivers.
        </p>
      </section>

      <SshProfileFields
        v-model:form="form"
        :fields="fields"
        :has-saved-password="!!editingProfile?.hasSshPassword"
      />
    </form>
    <template #footer>
      <BaseButton @click="open = false">Cancel</BaseButton>
      <ConnectionTestAction
        :profile-id="editingProfile?.id"
        :disabled="!editingProfile || !store.nativeAvailable"
      />
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

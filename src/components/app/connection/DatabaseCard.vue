<script setup>
import { computed, useId } from "vue";
import {
  pickCaCertificate,
  pickClientCertificate,
  pickClientKey,
} from "../../../lib/tauri/files.js";
import { TLS_MODES, tlsHint } from "../../../lib/tls.js";
import FormRow from "../../common/FormRow.vue";
import SelectInput from "../../common/SelectInput.vue";
import TextInput from "../../common/TextInput.vue";
import FilePick from "./FilePick.vue";
import SecretField from "./SecretField.vue";

/** Where the database is and how to sign in to it, including TLS. */
const form = defineModel("form", { type: Object, required: true });
defineProps({
  fields: { type: Object, required: true },
  hasSavedPassword: { type: Boolean, default: false },
});
const emit = defineEmits(["tls-touched"]);
const ids = { host: useId(), user: useId(), database: useId() };

const drivers = [
  { value: "postgres", label: "PostgreSQL" },
  { value: "mysql", label: "MySQL" },
  { value: "sqlserver", label: "SQL Server" },
];
const tlsMode = computed({
  get: () => form.value.tlsMode,
  set: (value) => {
    form.value.tlsMode = value;
    emit("tls-touched");
  },
});
const tlsNote = computed(() =>
  form.value.sshEnabled &&
  form.value.tlsMode === "verify-full" &&
  form.value.driver !== "sqlserver"
    ? "Through SSH, the certificate chain is checked but not the host name."
    : tlsHint(form.value.tlsMode, form.value.host),
);
</script>

<template>
  <section class="flex flex-col gap-2.5 rounded-lg bg-surface-hover/60 p-4">
    <FormRow label="Database type" :error="fields.driver">
      <SelectInput
        v-model="form.driver"
        class="w-48"
        :options="drivers"
        aria-label="Database type"
      />
    </FormRow>
    <FormRow
      label="Host"
      :for="ids.host"
      :error="fields.host || fields.port"
      :hint="
        form.sshEnabled ? 'As seen from the SSH server, usually 127.0.0.1.' : ''
      "
    >
      <div
        class="grid grid-cols-[minmax(0,1fr)_auto_5.5rem] items-center gap-2"
      >
        <TextInput :id="ids.host" v-model="form.host" placeholder="localhost" />
        <span class="text-sm text-fg-muted">Port</span>
        <TextInput
          v-model="form.port"
          input-mode="numeric"
          mono
          aria-label="Port"
        />
      </div>
    </FormRow>
    <FormRow label="User" :for="ids.user" :error="fields.username">
      <TextInput
        :id="ids.user"
        v-model="form.username"
        autocomplete="username"
      />
    </FormRow>
    <FormRow label="Password" :error="fields.databasePassword">
      <SecretField
        v-model="form.databasePassword"
        v-model:ask="form.askDatabasePassword"
        v-model:clear="form.clearDatabasePassword"
        label="Password"
        :has-saved="hasSavedPassword"
      />
    </FormRow>
    <FormRow
      label="Database"
      :for="ids.database"
      :error="fields.databaseName"
      hint="Optional. Leave empty to browse every database."
    >
      <TextInput :id="ids.database" v-model="form.databaseName" />
    </FormRow>
    <FormRow label="TLS" :error="fields.tlsMode" :hint="tlsNote">
      <SelectInput v-model="tlsMode" :options="TLS_MODES" aria-label="TLS" />
    </FormRow>
    <FormRow
      v-if="form.tlsMode !== 'disable'"
      label="TLS files"
      :error="fields.tlsCaPath || fields.tlsClientCert || fields.tlsClientKey"
      hint="Optional: a CA for servers signed by your own authority, and a client certificate and key if the server asks for one."
    >
      <div class="grid grid-cols-3 gap-2">
        <FilePick
          v-if="form.tlsMode === 'verify-full'"
          v-model="form.tlsCaPath"
          label="CA Cert"
          :pick="pickCaCertificate"
        />
        <FilePick
          v-model="form.tlsClientCert"
          label="Cert"
          :pick="pickClientCertificate"
        />
        <FilePick
          v-model="form.tlsClientKey"
          label="Key"
          :pick="pickClientKey"
        />
      </div>
    </FormRow>
  </section>
</template>

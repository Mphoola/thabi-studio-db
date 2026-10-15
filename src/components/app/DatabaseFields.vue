<script setup>
import { words } from "../../lib/platform.js";
import CheckboxInput from "../common/CheckboxInput.vue";
import PasswordInput from "../common/PasswordInput.vue";
import SelectInput from "../common/SelectInput.vue";
import TextInput from "../common/TextInput.vue";
import TlsFields from "./TlsFields.vue";

const form = defineModel("form", { type: Object, required: true });
defineProps({
  fields: { type: Object, required: true },
  hasSavedPassword: { type: Boolean, default: false },
  viaSsh: { type: Boolean, default: false },
});
defineEmits(["tls-touched"]);

const drivers = [
  { value: "postgres", label: "PostgreSQL" },
  { value: "mysql", label: "MySQL" },
  { value: "sqlserver", label: "SQL Server" },
];
</script>

<template>
  <section class="flex flex-col gap-3">
    <div>
      <h3 class="text-sm font-semibold text-fg">Database</h3>
      <p v-if="viaSsh" class="text-sm text-fg-muted">
        Still needed with SSH: the tunnel only carries the connection, then the
        database checks its own user name and password.
      </p>
    </div>
    <div class="grid grid-cols-2 gap-3">
      <SelectInput
        v-model="form.driver"
        label="Database type"
        :options="drivers"
        :error="fields.driver"
      />
      <div class="grid grid-cols-[1fr_88px] gap-3">
        <TextInput
          v-model="form.host"
          label="Host"
          placeholder="localhost"
          :hint="
            viaSsh ? 'As seen from the SSH server, usually localhost.' : ''
          "
          :error="fields.host"
        />
        <TextInput
          v-model="form.port"
          label="Port"
          input-mode="numeric"
          :error="fields.port"
          mono
        />
      </div>
      <TextInput
        v-model="form.username"
        label="User name"
        :error="fields.username"
        autocomplete="username"
      />
      <div class="flex flex-col gap-1">
        <PasswordInput
          v-if="!form.askDatabasePassword"
          v-model="form.databasePassword"
          label="Password"
          :hint="
            hasSavedPassword
              ? `Saved in ${words.credentialStore}. Leave blank to keep it.`
              : `Optional. Saved in ${words.credentialStore}.`
          "
          :error="fields.databasePassword"
        />
        <CheckboxInput
          v-model="form.askDatabasePassword"
          label="Ask for the password each time"
          :hint="`Nothing is saved; Thabi Studio asks when connecting.`"
        />
      </div>
      <TextInput
        v-model="form.databaseName"
        label="Database"
        hint="Optional. You can switch databases after connecting."
        :error="fields.databaseName"
      />
    </div>
    <CheckboxInput
      v-if="hasSavedPassword"
      v-model="form.clearDatabasePassword"
      label="Remove saved password"
    />
    <TlsFields
      v-model:form="form"
      :fields="fields"
      @touched="$emit('tls-touched')"
    />
  </section>
</template>

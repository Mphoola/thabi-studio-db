<script setup>
import CheckboxInput from "../common/CheckboxInput.vue";
import PasswordInput from "../common/PasswordInput.vue";
import SwitchInput from "../common/SwitchInput.vue";
import TextInput from "../common/TextInput.vue";

const form = defineModel("form", { type: Object, required: true });

defineProps({
  fields: { type: Object, required: true },
  hasSavedPassword: { type: Boolean, default: false },
});
</script>

<template>
  <section class="flex flex-col gap-3 border-t border-line pt-3">
    <SwitchInput
      v-model="form.sshEnabled"
      label="Use an SSH tunnel"
      description="Tunnels arrive in a later build. You can save SSH settings now, but connections with SSH on can't be opened yet."
    />
    <div v-if="form.sshEnabled" class="grid grid-cols-2 gap-3">
      <TextInput
        v-model="form.sshHost"
        label="SSH host"
        placeholder="bastion.example.com"
        :error="fields.sshHost"
        autocomplete="url"
      />
      <TextInput
        v-model="form.sshPort"
        label="SSH port"
        input-mode="numeric"
        :error="fields.sshPort"
        mono
      />
      <TextInput
        v-model="form.sshUsername"
        label="SSH user name"
        :error="fields.sshUsername"
        autocomplete="username"
      />
      <PasswordInput
        v-model="form.sshPassword"
        label="SSH password"
        :hint="
          hasSavedPassword
            ? 'Saved in Keychain. Leave blank to keep it.'
            : 'Optional. Stored in Keychain.'
        "
        :error="fields.sshPassword"
      />
      <CheckboxInput
        v-if="hasSavedPassword"
        v-model="form.clearSshPassword"
        label="Remove saved SSH password"
      />
    </div>
    <div v-else-if="hasSavedPassword" class="flex flex-col gap-2">
      <p class="text-sm text-fg-subtle">
        This profile has a saved SSH password, but SSH is currently off.
      </p>
      <CheckboxInput
        v-model="form.clearSshPassword"
        label="Remove saved SSH password"
      />
    </div>
    <p v-if="form.sshEnabled" class="text-sm text-fg-subtle">
      Password authentication only. SSH key authentication isn't available yet.
    </p>
  </section>
</template>

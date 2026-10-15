<script setup>
import { computed } from "vue";
import { KeyRound, X } from "lucide-vue-next";
import { pickSshKey } from "../../lib/tauri/files.js";
import BaseButton from "../common/BaseButton.vue";
import CheckboxInput from "../common/CheckboxInput.vue";
import IconButton from "../common/IconButton.vue";
import PasswordInput from "../common/PasswordInput.vue";
import SelectInput from "../common/SelectInput.vue";
import SwitchInput from "../common/SwitchInput.vue";
import TextInput from "../common/TextInput.vue";

const form = defineModel("form", { type: Object, required: true });

defineProps({
  fields: { type: Object, required: true },
  hasSavedPassword: { type: Boolean, default: false },
  hasSavedPassphrase: { type: Boolean, default: false },
});

const authModes = [
  { value: "password", label: "Password" },
  { value: "key", label: "Private key" },
  { value: "agent", label: "SSH agent" },
];
const keyFileName = computed(() => form.value.sshKeyPath.split("/").pop());

async function chooseKey() {
  const path = await pickSshKey();
  if (path) form.value.sshKeyPath = path;
}
</script>

<template>
  <section class="flex flex-col gap-3 border-t border-line pt-3">
    <SwitchInput
      v-model="form.sshEnabled"
      label="Connect through SSH"
      description="Reach a database on a private network through an SSH server."
    />
    <div v-if="form.sshEnabled" class="grid grid-cols-2 gap-3">
      <div class="grid grid-cols-[1fr_88px] gap-3">
        <TextInput
          v-model="form.sshHost"
          label="SSH host"
          placeholder="bastion.example.com"
          :error="fields.sshHost"
        />
        <TextInput
          v-model="form.sshPort"
          label="Port"
          input-mode="numeric"
          :error="fields.sshPort"
          mono
        />
      </div>
      <TextInput
        v-model="form.sshUsername"
        label="SSH user name"
        :error="fields.sshUsername"
        autocomplete="username"
      />
      <SelectInput
        v-model="form.sshAuthMode"
        label="Sign in with"
        :options="authModes"
        :error="fields.sshAuthMode"
        :hint="
          form.sshAuthMode === 'agent'
            ? 'Uses the keys loaded in ssh-agent (ssh-add -l).'
            : ''
        "
      />
      <PasswordInput
        v-if="form.sshAuthMode === 'password'"
        v-model="form.sshPassword"
        label="SSH password"
        :hint="
          hasSavedPassword
            ? 'Saved in Keychain. Leave blank to keep it.'
            : 'Saved in your Keychain.'
        "
        :error="fields.sshPassword"
      />
      <div v-else-if="form.sshAuthMode === 'key'" class="flex flex-col gap-1">
        <span class="text-sm font-medium text-fg">Private key</span>
        <div class="flex h-7 items-center gap-2">
          <template v-if="form.sshKeyPath">
            <KeyRound
              :size="14"
              :stroke-width="1.75"
              class="text-fg-subtle"
              aria-hidden="true"
            />
            <span class="min-w-0 flex-1 truncate" :title="form.sshKeyPath">
              {{ keyFileName }}
            </span>
            <IconButton
              :icon="X"
              size="sm"
              label="Remove Key File"
              @click="form.sshKeyPath = ''"
            />
          </template>
          <BaseButton v-else size="sm" @click="chooseKey">
            Choose Key File…
          </BaseButton>
        </div>
        <p v-if="fields.sshKeyPath" class="text-sm text-danger-text">
          {{ fields.sshKeyPath }}
        </p>
        <p v-else class="text-sm text-fg-subtle">
          Read when connecting; the key itself is never copied.
        </p>
      </div>
      <PasswordInput
        v-if="form.sshAuthMode === 'key'"
        v-model="form.sshKeyPassphrase"
        label="Key passphrase"
        :hint="
          hasSavedPassphrase
            ? 'Saved in Keychain. Leave blank to keep it.'
            : 'Only if the key has one. Saved in your Keychain.'
        "
        :error="fields.sshKeyPassphrase"
      />
      <CheckboxInput
        v-if="form.sshAuthMode === 'password' && hasSavedPassword"
        v-model="form.clearSshPassword"
        label="Remove saved SSH password"
      />
      <CheckboxInput
        v-if="form.sshAuthMode === 'key' && hasSavedPassphrase"
        v-model="form.clearSshKeyPassphrase"
        label="Remove saved passphrase"
      />
    </div>
  </section>
</template>

<script setup>
import { useId } from "vue";
import { ChevronDown } from "lucide-vue-next";
import { pickSshKey } from "../../../lib/tauri/files.js";
import FormRow from "../../common/FormRow.vue";
import SegmentedControl from "../../common/SegmentedControl.vue";
import TextInput from "../../common/TextInput.vue";
import SshConfigHint from "../SshConfigHint.vue";
import FilePick from "./FilePick.vue";
import SecretField from "./SecretField.vue";

/** Reaching the database through an SSH server ("Over SSH"). */
const form = defineModel("form", { type: Object, required: true });
defineProps({
  fields: { type: Object, required: true },
  hasSavedPassword: { type: Boolean, default: false },
  hasSavedPassphrase: { type: Boolean, default: false },
});
const ids = { host: useId(), user: useId(), keepalive: useId() };
const modes = [
  { value: "password", label: "Password" },
  { value: "key", label: "SSH key" },
  { value: "agent", label: "SSH agent" },
];
</script>

<template>
  <section class="flex flex-col gap-2.5">
    <button
      type="button"
      class="flex h-7 w-fit items-center gap-1.5 rounded-md px-3 text-sm font-medium transition-colors"
      :class="
        form.sshEnabled
          ? 'bg-accent text-accent-fg hover:bg-accent-hover'
          : 'border border-line-strong bg-surface text-fg hover:bg-surface-hover'
      "
      :aria-pressed="form.sshEnabled"
      @click="form.sshEnabled = !form.sshEnabled"
    >
      Over SSH
      <ChevronDown
        :size="14"
        :stroke-width="1.75"
        class="transition-transform"
        :class="form.sshEnabled && 'rotate-180'"
        aria-hidden="true"
      />
    </button>
    <div
      v-if="form.sshEnabled"
      class="flex flex-col gap-2.5 rounded-lg bg-surface-hover/60 p-4"
    >
      <FormRow
        label="Server"
        :for="ids.host"
        :error="fields.sshHost || fields.sshPort"
      >
        <div
          class="grid grid-cols-[minmax(0,1fr)_auto_5.5rem] items-center gap-2"
        >
          <TextInput
            :id="ids.host"
            v-model="form.sshHost"
            placeholder="bastion.example.com or a ~/.ssh/config Host"
          />
          <span class="text-sm text-fg-muted">Port</span>
          <TextInput
            v-model="form.sshPort"
            input-mode="numeric"
            mono
            aria-label="SSH port"
          />
        </div>
        <SshConfigHint v-model:form="form" />
      </FormRow>
      <FormRow label="User" :for="ids.user" :error="fields.sshUsername">
        <TextInput
          :id="ids.user"
          v-model="form.sshUsername"
          autocomplete="username"
        />
      </FormRow>
      <FormRow
        label="Sign in with"
        :error="fields.sshAuthMode"
        :hint="
          form.sshAuthMode === 'agent'
            ? 'Uses the keys loaded in ssh-agent (ssh-add -l).'
            : ''
        "
      >
        <SegmentedControl
          v-model="form.sshAuthMode"
          :options="modes"
          label="Sign in with"
        />
      </FormRow>
      <FormRow
        v-if="form.sshAuthMode === 'password'"
        label="Password"
        :error="fields.sshPassword"
      >
        <SecretField
          v-model="form.sshPassword"
          v-model:ask="form.askSshPassword"
          v-model:clear="form.clearSshPassword"
          label="SSH password"
          :has-saved="hasSavedPassword"
        />
      </FormRow>
      <template v-if="form.sshAuthMode === 'key'">
        <FormRow
          label="Private key"
          :error="fields.sshKeyPath"
          hint="Read when connecting; the key itself is never copied."
        >
          <FilePick
            v-model="form.sshKeyPath"
            label="Choose Key File"
            :pick="pickSshKey"
          />
        </FormRow>
        <FormRow
          label="Passphrase"
          :error="fields.sshKeyPassphrase"
          hint="Only if the key has one."
        >
          <SecretField
            v-model="form.sshKeyPassphrase"
            v-model:ask="form.askSshKeyPassphrase"
            v-model:clear="form.clearSshKeyPassphrase"
            label="Key passphrase"
            :has-saved="hasSavedPassphrase"
          />
        </FormRow>
      </template>
      <FormRow
        label="Keep-alive"
        :for="ids.keepalive"
        :error="fields.sshKeepaliveSeconds"
        hint="Seconds between keep-alives; 0 uses the default."
      >
        <TextInput
          :id="ids.keepalive"
          v-model="form.sshKeepaliveSeconds"
          class="w-24"
          input-mode="numeric"
          mono
        />
      </FormRow>
    </div>
  </section>
</template>

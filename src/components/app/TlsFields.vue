<script setup>
import { computed } from "vue";
import { FileKey, X } from "lucide-vue-next";
import { pickCaCertificate } from "../../lib/tauri/files.js";
import { TLS_MODES, tlsHint } from "../../lib/tls.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";
import SelectInput from "../common/SelectInput.vue";

const form = defineModel("form", { type: Object, required: true });
defineProps({ fields: { type: Object, required: true } });
const emit = defineEmits(["touched"]);

const mode = computed({
  get: () => form.value.tlsMode,
  set: (value) => {
    form.value.tlsMode = value;
    emit("touched");
  },
});
const caFileName = computed(() => form.value.tlsCaPath.split("/").pop() || "");

async function chooseCa() {
  const path = await pickCaCertificate();
  if (path) form.value.tlsCaPath = path;
}
</script>

<template>
  <div class="grid grid-cols-2 items-start gap-3">
    <SelectInput
      v-model="mode"
      label="TLS"
      :options="TLS_MODES"
      :hint="
        form.sshEnabled &&
        form.tlsMode === 'verify-full' &&
        form.driver !== 'sqlserver'
          ? 'Through SSH, the certificate chain is checked but not the host name.'
          : tlsHint(form.tlsMode, form.host)
      "
      :error="fields.tlsMode"
    />
    <div v-if="form.tlsMode === 'verify-full'" class="flex flex-col gap-1">
      <span class="text-sm font-medium text-fg">Trusted certificates</span>
      <div class="flex h-7 items-center gap-2">
        <template v-if="form.tlsCaPath">
          <FileKey
            :size="14"
            :stroke-width="1.75"
            class="text-fg-subtle"
            aria-hidden="true"
          />
          <span class="min-w-0 flex-1 truncate" :title="form.tlsCaPath">
            {{ caFileName }}
          </span>
          <IconButton
            :icon="X"
            size="sm"
            label="Use System Certificates"
            @click="form.tlsCaPath = ''"
          />
        </template>
        <template v-else>
          <span class="flex-1 text-fg-muted">System</span>
          <BaseButton size="sm" @click="chooseCa">Choose CA File…</BaseButton>
        </template>
      </div>
      <p v-if="fields.tlsCaPath" class="text-sm text-danger-text">
        {{ fields.tlsCaPath }}
      </p>
      <p v-else class="text-sm text-fg-subtle">
        For servers signed by your own certificate authority.
      </p>
    </div>
  </div>
</template>

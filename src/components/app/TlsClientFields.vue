<script setup>
import { computed } from "vue";
import { FileKey, ShieldCheck, X } from "lucide-vue-next";
import { pickClientCertificate, pickClientKey } from "../../lib/tauri/files.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";

/**
 * Mutual TLS: a client certificate and its private key, for servers that ask
 * the client to identify itself during the handshake. Only paths are stored;
 * the key is read at connect time and never copied, like an SSH key.
 */
const form = defineModel("form", { type: Object, required: true });
defineProps({ fields: { type: Object, default: () => ({}) } });

/** Where each chosen file is shown once a path has been set. */
const shown = (path) => path.split("/").pop();

const certName = computed(() => shown(form.value.tlsClientCert ?? ""));
const keyName = computed(() => shown(form.value.tlsClientKey ?? ""));

async function chooseCert() {
  const path = await pickClientCertificate();
  if (path) form.value.tlsClientCert = path;
}

async function chooseKey() {
  const path = await pickClientKey();
  if (path) form.value.tlsClientKey = path;
}
</script>

<template>
  <div v-if="form.tlsMode !== 'disable'" class="col-span-2 flex flex-col gap-1">
    <span class="text-sm font-medium text-fg">Client certificate</span>
    <div
      v-if="form.tlsClientCert || form.tlsClientKey"
      class="flex flex-col gap-1.5"
    >
      <div
        v-for="row in [
          {
            label: 'Certificate',
            value: form.tlsClientCert,
            name: certName,
            choose: chooseCert,
          },
          {
            label: 'Private key',
            value: form.tlsClientKey,
            name: keyName,
            choose: chooseKey,
          },
        ]"
        :key="row.label"
        class="flex items-center gap-2"
      >
        <span class="w-24 shrink-0 text-sm text-fg-muted">{{ row.label }}</span>
        <template v-if="row.value">
          <FileKey
            :size="14"
            :stroke-width="1.75"
            class="text-fg-subtle"
            aria-hidden="true"
          />
          <span
            class="min-w-0 flex-1 truncate font-mono text-xs"
            :title="row.value"
          >
            {{ row.name }}
          </span>
          <IconButton
            :icon="X"
            size="sm"
            :label="`Remove ${row.label}`"
            @click="
              row.value === form.tlsClientCert
                ? (form.tlsClientCert = '')
                : (form.tlsClientKey = '')
            "
          />
        </template>
        <span v-else class="flex-1 text-sm text-fg-subtle">Not chosen</span>
        <BaseButton size="sm" @click="row.choose">
          Choose {{ row.label }}…
        </BaseButton>
      </div>
      <p
        v-if="fields.tlsClientCert || fields.tlsClientKey"
        class="text-sm text-danger-text"
      >
        {{ fields.tlsClientCert || fields.tlsClientKey }}
      </p>
    </div>
    <template v-else>
      <p class="text-sm text-fg-subtle">
        For servers that ask the client to identify itself with a certificate.
      </p>
      <div>
        <BaseButton size="sm" :icon="ShieldCheck" @click="chooseCert">
          Choose Certificate…
        </BaseButton>
      </div>
    </template>
  </div>
</template>

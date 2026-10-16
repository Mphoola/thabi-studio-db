<script setup>
import { ref, watch } from "vue";
import { FileText } from "lucide-vue-next";
import { resolveSshAlias } from "../../lib/tauri/commands.js";
import BaseButton from "../common/BaseButton.vue";

/**
 * Shows what `~/.ssh/config` says about a `Host` alias the user typed, and
 * offers to fill those settings in. The file is only read, never written, and
 * only an alias that matches exactly is resolved.
 */
const form = defineModel("form", { type: Object, required: true });

/** The alias as resolved from the user's own SSH config, or null. */
const found = ref(null);
const source = ref("");
/** While typing, the previous answer is stale and must not be shown. */
const checking = ref(false);

watch(
  () => form.value.sshHost,
  async (host) => {
    const alias = (host ?? "").trim();
    found.value = null;
    if (!alias) return;
    checking.value = true;
    try {
      const resolved = await resolveSshAlias(alias);
      found.value = resolved?.config ?? null;
      source.value = resolved?.source ?? "";
    } catch {
      // An unreadable or absent config is not an error worth interrupting for.
      found.value = null;
    } finally {
      checking.value = false;
    }
  },
);

/** What the alias resolved to, as one line of plain text. */
function summarise() {
  const config = found.value;
  const host = config.hostname
    ? `${config.user ? `${config.user}@` : ""}${config.hostname}`
    : "";
  const parts = [host, config.port ? `port ${config.port}` : ""].filter(
    Boolean,
  );
  if (config.identityFile)
    parts.push(`key ${config.identityFile.split("/").pop()}`);
  if (config.proxyJump) parts.push(`via ${config.proxyJump}`);
  return parts.join(" · ");
}

function apply() {
  const config = found.value;
  if (!config) return;
  if (config.hostname) form.value.sshHost = config.hostname;
  if (config.user) form.value.sshUsername = config.user;
  if (config.port) form.value.sshPort = String(config.port);
  // An identity file only means anything with the key sign-in chosen.
  if (config.identityFile) {
    form.value.sshAuthMode = "key";
    form.value.sshKeyPath = config.identityFile;
  }
}
</script>

<template>
  <div
    v-if="found"
    class="flex flex-wrap items-center gap-2 rounded-md bg-surface-hover px-3 py-2 text-sm"
  >
    <FileText
      :size="14"
      :stroke-width="1.75"
      class="text-fg-subtle"
      aria-hidden="true"
    />
    <span class="min-w-0 flex-1">
      <span class="font-medium text-fg">{{ found.alias }}</span>
      <span class="text-fg-muted"> is {{ summarise() }} in </span>
      <span class="font-mono text-xs text-fg-subtle">{{ source }}</span>
      <span v-if="found.proxyJump" class="block text-xs text-fg-muted">
        Thabi Studio connects to {{ found.proxyJump }} first, then on to this
        server.
      </span>
    </span>
    <BaseButton size="sm" @click="apply">Use These Settings</BaseButton>
  </div>
</template>

<script setup>
import { ref } from "vue";
import {
  Copy,
  Download,
  Pencil,
  Plus,
  Search,
  Trash2,
  Unplug,
} from "lucide-vue-next";
import { confirm } from "../../composables/useConfirm.js";
import { useToasts } from "../../composables/useToasts.js";
import BaseButton from "../common/BaseButton.vue";
import CheckboxInput from "../common/CheckboxInput.vue";
import DropdownMenu from "../common/DropdownMenu.vue";
import EmptyState from "../common/EmptyState.vue";
import IconButton from "../common/IconButton.vue";
import InlineAlert from "../common/InlineAlert.vue";
import PasswordInput from "../common/PasswordInput.vue";
import SegmentedControl from "../common/SegmentedControl.vue";
import SelectInput from "../common/SelectInput.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";
import SwitchInput from "../common/SwitchInput.vue";
import TextInput from "../common/TextInput.vue";

// Development-only reference for the shared components. Not bundled in
// release builds (see AppShell.vue). Values here are examples.
const toasts = useToasts();
const host = ref("db.internal");
const port = ref("70000");
const password = ref("");
const engine = ref("postgres");
const tls = ref(true);
const remember = ref(false);
const segment = ref("data");
const lastAction = ref("");

const menuItems = [
  { id: "edit", label: "Edit…", icon: Pencil, shortcut: "Mod+E" },
  { id: "duplicate", label: "Duplicate", icon: Copy, shortcut: "Mod+D" },
  { separator: true },
  { id: "delete", label: "Delete…", icon: Trash2, danger: true },
];

async function askDelete() {
  const ok = await confirm({
    title: "Delete “Staging”?",
    message:
      "Its saved password is also removed from your keychain. This can't be undone.",
    confirmLabel: "Delete",
    destructive: true,
  });
  lastAction.value = ok ? "Confirmed" : "Cancelled";
}
</script>

<template>
  <div class="mx-auto flex max-w-3xl flex-col gap-10 p-8">
    <header class="flex flex-col gap-1">
      <p class="text-xs font-semibold tracking-wide text-fg-subtle uppercase">
        Developer
      </p>
      <h1 class="text-2xl font-semibold">Component Gallery</h1>
      <p class="text-fg-muted">
        Shared components in every state. Example values only.
      </p>
    </header>

    <section class="flex flex-col gap-3">
      <h2 class="text-lg font-semibold">Buttons</h2>
      <div class="flex flex-wrap items-center gap-2">
        <BaseButton variant="primary" :icon="Plus">New Connection…</BaseButton>
        <BaseButton :icon="Download">Export CSV…</BaseButton>
        <BaseButton variant="ghost">Cancel</BaseButton>
        <BaseButton variant="danger" :icon="Trash2">Delete</BaseButton>
        <BaseButton variant="primary" loading>Testing…</BaseButton>
        <BaseButton disabled>Disabled</BaseButton>
        <BaseButton size="sm">Small</BaseButton>
      </div>
      <div class="flex items-center gap-1">
        <IconButton :icon="Search" label="Search" shortcut="Mod+F" />
        <IconButton :icon="Unplug" label="Disconnect" />
        <IconButton :icon="Copy" label="Copy" size="sm" />
        <IconButton :icon="Trash2" label="Delete" disabled />
      </div>
    </section>

    <section class="flex flex-col gap-3">
      <h2 class="text-lg font-semibold">Inputs</h2>
      <div class="grid grid-cols-2 gap-4">
        <TextInput
          v-model="host"
          label="Host"
          hint="Hostname or IP address."
          mono
        />
        <TextInput
          v-model="port"
          label="Port"
          error="Port must be between 1 and 65535."
          mono
        />
        <SelectInput
          v-model="engine"
          label="Database"
          :options="[
            { value: 'postgres', label: 'PostgreSQL' },
            { value: 'mysql', label: 'MySQL' },
            { value: 'sqlserver', label: 'SQL Server' },
          ]"
        />
        <TextInput label="Filter" placeholder="Filter tables" :icon="Search" />
        <TextInput label="Disabled" placeholder="Not editable" disabled />
        <PasswordInput
          v-model="password"
          label="Password"
          hint="Stored securely in Keychain."
        />
      </div>
    </section>

    <section class="flex flex-col gap-3">
      <h2 class="text-lg font-semibold">Toggles</h2>
      <div class="flex max-w-md flex-col gap-3">
        <SwitchInput
          v-model="tls"
          label="Verify server certificate"
          description="Recommended. Turn off only for a server you trust."
        />
        <CheckboxInput v-model="remember" label="Save password in Keychain" />
        <SegmentedControl
          v-model="segment"
          label="Table view"
          :options="[
            { value: 'data', label: 'Data' },
            { value: 'structure', label: 'Structure' },
          ]"
        />
      </div>
    </section>

    <section class="flex flex-col gap-3">
      <h2 class="text-lg font-semibold">Menus, dialogs and toasts</h2>
      <div class="flex flex-wrap items-center gap-2">
        <DropdownMenu
          :items="menuItems"
          label="Connection actions"
          @select="(id) => (lastAction = `Menu: ${id}`)"
        >
          <template #trigger="{ props }">
            <BaseButton v-bind="props">Actions</BaseButton>
          </template>
        </DropdownMenu>
        <BaseButton @click="askDelete">Confirm…</BaseButton>
        <BaseButton @click="toasts.success('Copied 3 rows.')"
          >Success Toast</BaseButton
        >
        <BaseButton
          @click="
            toasts.error(
              'Couldn’t connect to “Staging”',
              'The server refused the connection on port 5432.',
            )
          "
        >
          Error Toast
        </BaseButton>
        <span v-if="lastAction" class="text-sm text-fg-subtle">{{
          lastAction
        }}</span>
      </div>
    </section>

    <section class="flex flex-col gap-3">
      <h2 class="text-lg font-semibold">States</h2>
      <div class="flex items-center gap-2 text-fg-muted">
        <SpinnerIcon :size="14" /> Running… 3.2 s
      </div>
      <InlineAlert
        tone="danger"
        title="Query failed on “Staging” (orders_db)"
        message='ERROR: column "totl" does not exist (line 2)'
        mono
      >
        <BaseButton size="sm" :icon="Copy">Copy Message</BaseButton>
      </InlineAlert>
      <InlineAlert
        tone="warning"
        title="This statement can change data in “orders_db”."
      />
      <InlineAlert
        tone="success"
        title="Connected to PostgreSQL 16.4 in 38 ms."
      />
      <InlineAlert tone="info" title="First 500 rows fetched." />
      <div class="rounded-lg border border-line">
        <EmptyState
          size="sm"
          :icon="Search"
          title="No tables in “public”"
          description="Create a table or pick another schema."
        />
      </div>
    </section>
  </div>
</template>

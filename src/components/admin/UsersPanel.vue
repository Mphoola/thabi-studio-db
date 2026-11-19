<script setup>
import { computed, ref, watch } from "vue";
import {
  KeyRound,
  Plus,
  ShieldPlus,
  Trash2,
  UserPlus,
  X,
} from "lucide-vue-next";
import { toUserDetail } from "../../lib/errors.js";
import { listUsers, userGrants } from "../../lib/tauri/commands.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";
import InlineAlert from "../common/InlineAlert.vue";
import UserActionDialog from "./UserActionDialog.vue";

/**
 * Users and roles on the server (or, on SQL Server, in this database), what
 * each may do, and the changes to make: each one shown as SQL first.
 */
const props = defineProps({
  profileId: { type: String, required: true },
  database: { type: String, required: true },
  engine: { type: String, default: "postgres" },
  readOnly: { type: Boolean, default: false },
});

const users = ref([]);
const selected = ref(null);
const grants = ref([]);
const error = ref("");
const dialog = ref({ open: false, kind: "createUser", role: "" });

async function load() {
  error.value = "";
  try {
    users.value = await listUsers(props.profileId, props.database);
    if (selected.value)
      selected.value =
        users.value.find(
          (u) =>
            u.name === selected.value.name && u.host === selected.value.host,
        ) ?? null;
  } catch (failure) {
    error.value = toUserDetail(failure);
  }
}

watch(selected, async (user) => {
  grants.value = [];
  if (!user) return;
  try {
    grants.value = await userGrants(
      props.profileId,
      props.database,
      user.name,
      user.host,
    );
  } catch (failure) {
    grants.value = [`-- ${toUserDetail(failure)}`];
  }
});
load();

const label = (user) => (user.host ? `${user.name}@${user.host}` : user.name);
const key = (user) => `${user.name}@${user.host}`;
const scopeNote = computed(() =>
  props.engine === "sqlserver"
    ? `Users in “${props.database}”, and the logins they belong to.`
    : "Everyone on this server who can be given rights.",
);

function act(kind, role = "") {
  dialog.value = { open: true, kind, role };
}
</script>

<template>
  <div class="flex min-h-0 flex-1">
    <aside class="flex w-64 shrink-0 flex-col border-r border-line">
      <div class="flex items-center justify-between px-3 py-2">
        <span class="text-xs text-fg-subtle">{{ scopeNote }}</span>
        <IconButton
          v-if="!readOnly"
          :icon="UserPlus"
          label="New User"
          @click="((selected = null), act('createUser'))"
        />
      </div>
      <InlineAlert
        v-if="error"
        class="mx-3"
        tone="danger"
        title="Couldn't read the users"
        :message="error"
      />
      <ul class="min-h-0 flex-1 overflow-auto px-1.5 pb-2" role="listbox">
        <li v-for="user in users" :key="key(user)">
          <button
            type="button"
            role="option"
            :aria-selected="selected && key(selected) === key(user)"
            class="flex w-full items-center gap-1.5 rounded-md px-2 py-1 text-left text-sm"
            :class="
              selected && key(selected) === key(user)
                ? 'bg-surface-active text-fg'
                : 'text-fg-muted hover:bg-surface-hover'
            "
            @click="selected = user"
          >
            <span class="truncate font-mono text-xs">{{ label(user) }}</span>
            <span class="ml-auto text-xs text-fg-subtle">{{
              user.superuser ? "super" : user.kind
            }}</span>
          </button>
        </li>
      </ul>
    </aside>
    <section v-if="selected" class="min-w-0 flex-1 overflow-auto p-4">
      <header class="flex items-center gap-2">
        <h3 class="font-mono text-base">{{ label(selected) }}</h3>
        <span class="text-xs text-fg-subtle">{{
          selected.canLogin ? "can sign in" : "can't sign in"
        }}</span>
        <div v-if="!readOnly" class="ml-auto flex gap-1.5">
          <BaseButton size="sm" :icon="KeyRound" @click="act('setPassword')"
            >Password…</BaseButton
          >
          <BaseButton size="sm" :icon="ShieldPlus" @click="act('grant')"
            >Grant…</BaseButton
          >
          <BaseButton size="sm" @click="act('revoke')">Revoke…</BaseButton>
          <BaseButton
            size="sm"
            variant="danger"
            :icon="Trash2"
            @click="act('dropUser')"
            >Drop…</BaseButton
          >
        </div>
      </header>
      <h4 class="mt-4 mb-1 text-xs font-semibold text-fg-muted">Roles</h4>
      <div class="flex flex-wrap items-center gap-1.5">
        <span
          v-for="role in selected.roles"
          :key="role"
          class="flex h-6 items-center gap-1 rounded-md border border-line px-2 font-mono text-xs"
        >
          {{ role }}
          <button
            v-if="!readOnly"
            type="button"
            class="text-fg-subtle hover:text-fg"
            :aria-label="`Remove ${role}`"
            @click="act('revokeRole', role)"
          >
            <X :size="12" :stroke-width="1.75" aria-hidden="true" />
          </button>
        </span>
        <span v-if="!selected.roles.length" class="text-sm text-fg-subtle"
          >None.</span
        >
        <IconButton
          v-if="!readOnly"
          :icon="Plus"
          size="sm"
          label="Add to a Role"
          @click="act('grantRole')"
        />
      </div>
      <h4 class="mt-4 mb-1 text-xs font-semibold text-fg-muted">
        What it may do
      </h4>
      <pre
        class="rounded-md bg-surface-hover p-2 font-mono text-xs whitespace-pre-wrap"
        >{{ grants.join("\n") || "Nothing granted directly." }}</pre>
    </section>
    <p v-else class="m-auto text-sm text-fg-subtle">Choose a user.</p>
    <UserActionDialog
      v-model:open="dialog.open"
      :kind="dialog.kind"
      :role="dialog.role"
      :profile-id="profileId"
      :database="database"
      :engine="engine"
      :user="selected"
      @done="load"
    />
  </div>
</template>

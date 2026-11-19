<script setup>
import { computed, ref, watch } from "vue";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail } from "../../lib/errors.js";
import {
  applyUserAction,
  previewUserAction,
} from "../../lib/tauri/commands.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import CheckboxInput from "../common/CheckboxInput.vue";
import InlineAlert from "../common/InlineAlert.vue";
import PasswordInput from "../common/PasswordInput.vue";
import TextInput from "../common/TextInput.vue";

/**
 * One change to a user: create, set password, grant or revoke privileges,
 * add or remove a role, or drop. The SQL is shown before it runs, with the
 * password masked; a drop asks for the name typed.
 */
const open = defineModel("open", { type: Boolean, default: false });
const props = defineProps({
  /** "createUser", "setPassword", "grant", "revoke", "grantRole",
   *  "revokeRole" or "dropUser" */
  kind: { type: String, required: true },
  profileId: { type: String, required: true },
  database: { type: String, required: true },
  engine: { type: String, default: "postgres" },
  /** The user acted on; empty for a new one. */
  user: { type: Object, default: null },
  /** A role to remove, for revokeRole. */
  role: { type: String, default: "" },
});
const emit = defineEmits(["done"]);
const toasts = useToasts();

const PRIVILEGES = ["SELECT", "INSERT", "UPDATE", "DELETE", "ALL"];
const form = ref({});
const preview = ref(null);
const error = ref("");
const busy = ref(false);

watch(open, (value) => {
  if (!value) return;
  form.value = {
    name: props.user?.name ?? "",
    host: props.user?.host || (props.engine === "mysql" ? "%" : ""),
    password: "",
    role: props.role,
    privileges: { SELECT: true },
    schema:
      props.engine === "sqlserver"
        ? "dbo"
        : props.engine === "postgres"
          ? "public"
          : "",
    table: "",
    typed: "",
  };
  preview.value = null;
  error.value = "";
});

const titles = {
  createUser: "New User",
  setPassword: "Change Password",
  grant: "Grant Privileges",
  revoke: "Revoke Privileges",
  grantRole: "Add to a Role",
  revokeRole: "Remove from a Role",
  dropUser: "Drop User",
};
const action = computed(() => {
  const f = form.value;
  const base = { kind: props.kind, name: f.name?.trim(), host: f.host ?? "" };
  switch (props.kind) {
    case "createUser":
      return { ...base, password: f.password, database: props.database };
    case "setPassword":
      return { ...base, password: f.password };
    case "grantRole":
    case "revokeRole":
      return { ...base, role: f.role?.trim() };
    case "grant":
    case "revoke":
      return {
        ...base,
        privileges: PRIVILEGES.filter((p) => f.privileges?.[p]),
        scope: {
          database: props.database,
          schema: f.schema?.trim() || null,
          table: f.table?.trim() || null,
        },
      };
    default:
      return base;
  }
});
const dropping = computed(() => props.kind === "dropUser");
const ready = computed(
  () =>
    Boolean(preview.value) &&
    (!dropping.value || form.value.typed?.trim() === form.value.name),
);

async function review() {
  error.value = "";
  try {
    preview.value = await previewUserAction(props.profileId, action.value);
  } catch (failure) {
    preview.value = null;
    error.value = toUserDetail(failure);
  }
}

async function run() {
  busy.value = true;
  try {
    await applyUserAction(
      props.profileId,
      `users:${props.database}`,
      props.database,
      action.value,
    );
    toasts.success(titles[props.kind], form.value.name);
    emit("done");
    open.value = false;
  } catch (failure) {
    error.value = toUserDetail(failure);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <BaseDialog v-model:open="open" size="md" :title="titles[kind]">
    <form class="flex flex-col gap-3" @submit.prevent="review">
      <div class="flex gap-2">
        <TextInput
          v-model="form.name"
          class="flex-1"
          label="User"
          :disabled="kind !== 'createUser'"
          autocomplete="off"
        />
        <TextInput
          v-if="engine === 'mysql'"
          v-model="form.host"
          class="w-32"
          label="Host"
          :disabled="kind !== 'createUser'"
        />
      </div>
      <PasswordInput
        v-if="kind === 'createUser' || kind === 'setPassword'"
        v-model="form.password"
        label="Password"
      />
      <TextInput
        v-if="kind === 'grantRole' || kind === 'revokeRole'"
        v-model="form.role"
        label="Role"
        :disabled="kind === 'revokeRole'"
        :placeholder="engine === 'sqlserver' ? 'db_datareader' : 'reporting'"
      />
      <template v-if="kind === 'grant' || kind === 'revoke'">
        <div class="flex flex-wrap gap-3">
          <CheckboxInput
            v-for="privilege in PRIVILEGES"
            :key="privilege"
            v-model="form.privileges[privilege]"
            :label="privilege"
          />
        </div>
        <div class="flex gap-2">
          <TextInput
            v-if="engine !== 'mysql'"
            v-model="form.schema"
            class="w-40"
            label="Schema"
          />
          <TextInput
            v-model="form.table"
            class="flex-1"
            label="Table"
            :placeholder="`every table in ${engine === 'mysql' ? database : 'the schema'}`"
          />
        </div>
      </template>
      <BaseButton class="self-start" type="submit">Review SQL</BaseButton>
      <ol v-if="preview" class="flex flex-col gap-1.5">
        <li
          v-for="(statement, i) in preview"
          :key="i"
          class="rounded-md bg-surface-hover p-2 font-mono text-xs whitespace-pre-wrap"
        >
          {{ statement }}
        </li>
      </ol>
      <template v-if="dropping && preview">
        <p class="text-sm text-fg-muted">
          <template v-if="engine === 'postgres'"
            >What it owns in “{{ database }}” becomes yours, and its privileges
            are revoked, before it is dropped.
          </template>
          Type <strong class="font-mono">{{ form.name }}</strong> to drop it.
        </p>
        <TextInput
          v-model="form.typed"
          mono
          :aria-label="`Type ${form.name}`"
        />
      </template>
      <InlineAlert
        v-if="error"
        tone="danger"
        title="Not done"
        :message="error"
      />
    </form>
    <template #footer>
      <BaseButton @click="open = false">Cancel</BaseButton>
      <BaseButton
        :variant="dropping ? 'danger' : 'primary'"
        :disabled="!ready"
        :loading="busy"
        @click="run"
        >Run</BaseButton
      >
    </template>
  </BaseDialog>
</template>

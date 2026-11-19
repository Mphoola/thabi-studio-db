<script setup>
import { computed, ref, watch } from "vue";
import {
  applyContainerAction,
  previewContainerAction,
} from "../../lib/tauri/commands.js";
import { toUserDetail, toUserMessage } from "../../lib/errors.js";
import { useToasts } from "../../composables/useToasts.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import InlineAlert from "../common/InlineAlert.vue";
import TextInput from "../common/TextInput.vue";
import DdlReviewDialog from "./DdlReviewDialog.vue";

/**
 * Creates or drops a database or schema. A new one asks for its name; a
 * drop asks for the name typed, because nothing here can bring it back. Both
 * then show the SQL before anything runs.
 */
const open = defineModel("open", { type: Boolean, default: false });
const props = defineProps({
  /** { profileId, kind: "createDatabase" | "dropDatabase" | "createSchema"
   *  | "dropSchema", database, name? } */
  target: { type: Object, required: true },
});
const emit = defineEmits(["done"]);

const toasts = useToasts();
const name = ref("");
const typed = ref("");
const error = ref("");
const preview = ref(null);
const reviewing = ref(false);
const working = ref(false);

const dropping = computed(() => props.target.kind.startsWith("drop"));
const noun = computed(() =>
  props.target.kind.endsWith("Database") ? "database" : "schema",
);
const title = computed(() =>
  dropping.value
    ? `Drop ${noun.value} “${props.target.name}”`
    : `New ${noun.value === "database" ? "Database" : "Schema"}`,
);
const ready = computed(() =>
  dropping.value
    ? typed.value.trim() === props.target.name
    : Boolean(name.value.trim()),
);
const tabId = computed(
  () => `container:${props.target.database ?? ""}:${props.target.kind}`,
);

watch(open, (value) => {
  if (!value) return;
  name.value = "";
  typed.value = "";
  error.value = "";
  preview.value = null;
});

function action() {
  return {
    kind: props.target.kind,
    name: dropping.value ? props.target.name : name.value.trim(),
    ...(noun.value === "schema" ? { database: props.target.database } : {}),
  };
}

async function review() {
  if (!ready.value) return;
  working.value = true;
  error.value = "";
  try {
    preview.value = await previewContainerAction(
      props.target.profileId,
      tabId.value,
      action(),
    );
    reviewing.value = true;
  } catch (failure) {
    error.value = toUserMessage(failure);
  } finally {
    working.value = false;
  }
}

async function run() {
  working.value = true;
  const what = action();
  try {
    await applyContainerAction(props.target.profileId, tabId.value, what);
    toasts.success(
      dropping.value
        ? `Dropped ${noun.value} “${what.name}”`
        : `Created ${noun.value} “${what.name}”`,
    );
    reviewing.value = false;
    open.value = false;
    emit("done");
  } catch (failure) {
    toasts.error(
      `Couldn’t ${dropping.value ? "drop" : "create"} it`,
      toUserDetail(failure),
    );
  } finally {
    working.value = false;
  }
}
</script>

<template>
  <BaseDialog v-model:open="open" size="sm" :title="title">
    <form class="flex flex-col gap-3" @submit.prevent="review">
      <template v-if="dropping">
        <p class="text-sm text-fg-muted">
          <template v-if="noun === 'database'">
            Every table, view and row in it is removed. This can't be undone.
          </template>
          <template v-else>
            The server refuses to drop a schema that still has objects in it.
          </template>
          Type <strong class="font-mono">{{ target.name }}</strong> to continue.
        </p>
        <TextInput
          v-model="typed"
          mono
          :aria-label="`Type ${target.name} to confirm`"
          autocomplete="off"
        />
      </template>
      <TextInput
        v-else
        v-model="name"
        :label="`${noun === 'database' ? 'Database' : 'Schema'} name`"
        autocomplete="off"
      />
      <InlineAlert
        v-if="error"
        tone="danger"
        title="Not quite"
        :message="error"
      />
    </form>
    <template #footer>
      <BaseButton size="sm" @click="open = false">Cancel</BaseButton>
      <BaseButton
        size="sm"
        :variant="dropping ? 'danger' : 'primary'"
        :disabled="!ready"
        :loading="working && !reviewing"
        @click="review"
        >Review SQL</BaseButton
      >
    </template>
  </BaseDialog>
  <DdlReviewDialog
    v-model:open="reviewing"
    :title="title"
    :preview="preview"
    :applying="working"
    @close="reviewing = false"
    @apply="run"
  />
</template>

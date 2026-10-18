<script setup>
import { computed } from "vue";
import { TriangleAlert } from "lucide-vue-next";
import InlineAlert from "../common/InlineAlert.vue";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";

/**
 * The DDL a change will run, before it runs.
 *
 * The text here is the text that is sent — both come from the same call in
 * Rust, so what was read cannot differ from what happens. That is the whole
 * point of showing it, and why it is shown rather than summarised.
 *
 * The note under it is not decoration. MySQL commits each DDL statement on its
 * own, so a failure part way through leaves the earlier ones applied; saying
 * "transaction" on that engine would be a promise the database does not keep.
 */
const open = defineModel("open", { type: Boolean, default: false });
const props = defineProps({
  /** What is being changed, for the heading: "thabi_it.public.orders". */
  title: { type: String, default: "" },
  /** The preview from `preview_table_ddl`. */
  preview: { type: Object, default: null },
  loading: { type: Boolean, default: false },
  applying: { type: Boolean, default: false },
  /** Shown instead of the statements when something went wrong reading them. */
  error: { type: String, default: "" },
});
const emit = defineEmits(["close", "apply"]);

const statements = computed(() => props.preview?.statements ?? []);
/** Whether a preview has arrived. The body renders even while the dialog is
 * closed, so every branch has to cope with there not being one yet. */
const ready = computed(() => Boolean(props.preview));
const empty = computed(() => ready.value && !statements.value.length);
</script>

<template>
  <BaseDialog v-model:open="open" size="lg" :title="title || 'Review changes'">
    <div class="flex max-h-[60vh] flex-col gap-3 overflow-auto">
      <div
        v-if="loading"
        class="flex items-center gap-2 py-4 text-sm text-fg-muted"
      >
        <SpinnerIcon :size="14" /> Working out the SQL…
      </div>
      <InlineAlert
        v-else-if="error"
        tone="danger"
        title="Couldn't work out the SQL"
        :message="error"
      />
      <p v-else-if="!ready" class="text-sm text-fg-muted">Nothing to review.</p>
      <p v-else-if="empty" class="text-sm text-fg-muted">
        Nothing to run. Make a change first.
      </p>
      <template v-else>
        <ol class="flex flex-col gap-2">
          <li v-for="(statement, index) in statements" :key="index">
            <pre
              class="selectable overflow-auto rounded-md bg-surface-hover p-2.5 font-mono text-xs whitespace-pre-wrap text-fg"
              >{{ statement }}</pre>
          </li>
        </ol>
        <InlineAlert
          v-if="preview?.note"
          :tone="preview?.transactional ? 'info' : 'warning'"
          :title="
            preview?.transactional ? 'All or nothing' : 'Not all or nothing'
          "
          :message="preview?.note || ''"
        >
          <TriangleAlert
            v-if="preview && !preview.transactional"
            :size="14"
            aria-hidden="true"
          />
        </InlineAlert>
      </template>
    </div>
    <template #footer>
      <BaseButton size="sm" @click="emit('close')">Cancel</BaseButton>
      <BaseButton
        size="sm"
        variant="primary"
        :loading="applying"
        :disabled="loading || !ready || !statements.length"
        @click="emit('apply')"
      >
        {{ applying ? "Running…" : "Run these" }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

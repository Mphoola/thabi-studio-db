<script setup>
/**
 * The form: where the backup goes, what goes in it, and the parts most people
 * never need.
 *
 * Out of the dialog because it is a form and the dialog is a conversation: one
 * chooses, the other confirms, shows progress and reports the result.
 */
import { computed, ref } from "vue";
import BaseButton from "../common/BaseButton.vue";
import CheckboxInput from "../common/CheckboxInput.vue";
import InlineAlert from "../common/InlineAlert.vue";
import TextInput from "../common/TextInput.vue";
import ExportTablePicker from "./ExportTablePicker.vue";

const props = defineProps({
  destination: { type: String, default: "" },
  database: { type: String, required: true },
  schema: { type: String, default: "" },
  engine: { type: String, default: "" },
  structure: { type: Boolean, required: true },
  rows: { type: Boolean, required: true },
  limit: { type: String, default: "" },
  available: { type: Array, default: () => [] },
  picked: { type: Array, default: () => [] },
});
const emit = defineEmits([
  "change-destination",
  "update:structure",
  "update:rows",
  "update:limit",
  "toggle-table",
  "all-tables",
  "no-tables",
]);

const advanced = ref(false);
/** MySQL will state its own CREATE TABLE; the other two cannot. */
const rebuilt = computed(
  () => props.engine === "postgres" || props.engine === "sqlserver",
);
</script>

<template>
  <div class="space-y-4">
    <!-- The destination comes first. Everything else is a choice about the
         file; this is where it lands, and it should not be a question asked
         after the decision. -->
    <section class="rounded-md border border-line bg-surface p-2">
      <div class="flex items-start gap-2">
        <div class="min-w-0 flex-1">
          <h3 class="text-xs font-semibold text-fg-muted">
            Save the backup to
          </h3>
          <p
            class="mt-0.5 truncate font-mono text-xs text-fg-muted"
            :title="destination"
          >
            {{ destination || "Choosing a folder…" }}
          </p>
        </div>
        <BaseButton size="sm" @click="emit('change-destination')"
          >Change…</BaseButton
        >
      </div>
    </section>

    <section>
      <h3 class="mb-1.5 text-xs font-semibold text-fg-muted">
        What to include
      </h3>
      <div class="flex gap-4">
        <CheckboxInput
          :model-value="structure"
          label="Table structure"
          @update:model-value="emit('update:structure', $event)"
        />
        <CheckboxInput
          :model-value="rows"
          label="All rows"
          @update:model-value="emit('update:rows', $event)"
        />
      </div>
      <p v-if="rebuilt" class="mt-1.5 text-xs text-fg-subtle">
        {{ engine === "postgres" ? "PostgreSQL" : "SQL Server" }} cannot print
        its own <code>CREATE TABLE</code>, so the table structure is rebuilt
        here. Triggers and check constraints are not included.
      </p>
      <p v-else class="mt-1.5 text-xs text-fg-subtle">
        The table structure is copied exactly as the server writes it.
      </p>
    </section>

    <InlineAlert tone="warning">
      This file replaces tables of the same name when it is run against a
      database that already has them.
    </InlineAlert>

    <section>
      <button
        type="button"
        class="text-xs text-fg-muted hover:text-fg"
        @click="advanced = !advanced"
      >
        {{ advanced ? "Hide" : "Choose tables or limit rows" }}
      </button>

      <div v-if="advanced" class="mt-2 space-y-3">
        <ExportTablePicker
          v-if="available.length > 1"
          :available="available"
          :picked="picked"
          @toggle="emit('toggle-table', $event)"
          @all="emit('all-tables')"
          @none="emit('no-tables')"
        />
        <div>
          <TextInput
            :model-value="limit"
            label="Row limit, per table (optional)"
            placeholder="all rows"
            inputmode="numeric"
            @update:model-value="emit('update:limit', $event)"
          />
          <p class="mt-1 text-xs text-fg-subtle">
            A limit makes the file a sample rather than a backup.
          </p>
        </div>
      </div>
    </section>
  </div>
</template>

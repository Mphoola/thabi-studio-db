<script setup>
import { ref, useId } from "vue";
import { ChevronRight } from "lucide-vue-next";
import { useAppSettings } from "../../../composables/useAppSettings.js";
import { describeLimit } from "../../../lib/limits.js";
import FormRow from "../../common/FormRow.vue";
import TextInput from "../../common/TextInput.vue";
import ConnectionUrlField from "../../connections/ConnectionUrlField.vue";

/** Settings most connections never need, folded away until asked for. */
const form = defineModel("form", { type: Object, required: true });
const props = defineProps({ fields: { type: Object, required: true } });
const emit = defineEmits(["fill-url"]);
const { settings } = useAppSettings();
const ids = {
  group: useId(),
  startup: useId(),
  timeout: useId(),
  rows: useId(),
};
// Opens by itself when one of its settings has an error to show.
const open = ref(false);
const hasError = () =>
  ["groupName", "startupCommands", "queryTimeoutSeconds", "maxResultRows"].some(
    (key) => props.fields[key],
  );
defineExpose({ reveal: () => (open.value = true) });
</script>

<template>
  <section class="flex flex-col gap-2.5">
    <button
      type="button"
      class="flex w-fit items-center gap-1 text-sm font-medium text-fg-muted hover:text-fg"
      :aria-expanded="open || hasError()"
      @click="open = !open"
    >
      <ChevronRight
        :size="14"
        :stroke-width="1.75"
        class="transition-transform"
        :class="(open || hasError()) && 'rotate-90'"
        aria-hidden="true"
      />
      More options
    </button>
    <div
      v-if="open || hasError()"
      class="flex flex-col gap-2.5 rounded-lg bg-surface-hover/60 p-4"
    >
      <FormRow label="From a URL">
        <ConnectionUrlField bare @fill="emit('fill-url', $event)" />
      </FormRow>
      <FormRow
        label="Group"
        :for="ids.group"
        :error="fields.groupName"
        hint="Leave empty to keep it with its environment."
      >
        <TextInput
          :id="ids.group"
          v-model="form.groupName"
          placeholder="Work"
        />
      </FormRow>
      <FormRow
        label="Startup"
        :for="ids.startup"
        :error="fields.startupCommands"
        hint="Statements run on every new session, one per line. If one fails, the connection isn't opened."
      >
        <TextInput
          :id="ids.startup"
          v-model="form.startupCommands"
          multiline
          :rows="3"
          mono
          placeholder="SET time_zone = '+00:00'"
        />
      </FormRow>
      <FormRow
        label="Query timeout"
        :for="ids.timeout"
        :error="fields.queryTimeoutSeconds"
        :hint="`Seconds; 0 uses the app default (${describeLimit(settings.queryTimeoutSeconds, 'seconds')}).`"
      >
        <TextInput
          :id="ids.timeout"
          v-model="form.queryTimeoutSeconds"
          class="w-28"
          input-mode="numeric"
          mono
        />
      </FormRow>
      <FormRow
        label="Rows per result"
        :for="ids.rows"
        :error="fields.maxResultRows"
        :hint="`0 uses the app default (${describeLimit(settings.maxResultRows, 'rows')}).`"
      >
        <TextInput
          :id="ids.rows"
          v-model="form.maxResultRows"
          class="w-28"
          input-mode="numeric"
          mono
        />
      </FormRow>
    </div>
  </section>
</template>

<script setup>
import { useId } from "vue";
import FormField from "./FormField.vue";
import { borderClass, describedBy, fieldClass } from "./fieldClasses.js";

const props = defineProps({
  label: { type: String, default: "" },
  hint: { type: String, default: "" },
  error: { type: String, default: "" },
  type: { type: String, default: "text" },
  placeholder: { type: String, default: "" },
  /** A lucide icon shown inside the field on the left. */
  icon: { type: [Object, Function], default: null },
  mono: { type: Boolean, default: false },
  /** Accessible name when there's no visible label. */
  ariaLabel: { type: String, default: "" },
  disabled: { type: Boolean, default: false },
  id: { type: String, default: "" },
  autocomplete: { type: String, default: "off" },
  inputMode: { type: String, default: "" },
  /** Id of a <datalist> with suggestions. */
  list: { type: String, default: "" },
  /** Renders a <textarea> of `rows` lines, for several lines of text. */
  multiline: { type: Boolean, default: false },
  rows: { type: Number, default: 3 },
});

const model = defineModel({ type: [String, Number], default: "" });
const autoId = useId();
const inputId = props.id || autoId;
</script>

<template>
  <FormField :id="inputId" :label="label" :hint="hint" :error="error">
    <div class="relative">
      <component
        :is="icon"
        v-if="icon"
        :size="14"
        :stroke-width="1.75"
        class="pointer-events-none absolute top-1/2 left-2 -translate-y-1/2 text-fg-subtle"
        aria-hidden="true"
      />
      <textarea
        v-if="multiline"
        :id="inputId"
        v-model="model"
        :rows="rows"
        :placeholder="placeholder"
        :disabled="disabled"
        :aria-label="ariaLabel || undefined"
        :aria-invalid="!!error || undefined"
        :aria-describedby="describedBy(inputId, { error, hint })"
        :class="[fieldClass, borderClass(error), mono && 'font-mono']"
        spellcheck="false"
      />
      <input
        v-else
        :id="inputId"
        v-model="model"
        :type="type"
        :placeholder="placeholder"
        :disabled="disabled"
        :autocomplete="autocomplete"
        :inputmode="inputMode || undefined"
        :list="list || undefined"
        :aria-label="ariaLabel || undefined"
        :aria-invalid="!!error || undefined"
        :aria-describedby="describedBy(inputId, { error, hint })"
        :class="[
          fieldClass,
          borderClass(error),
          icon && 'pl-7',
          mono && 'font-mono',
        ]"
        spellcheck="false"
      />
    </div>
  </FormField>
</template>

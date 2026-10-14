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
  disabled: { type: Boolean, default: false },
  id: { type: String, default: "" },
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
      <input
        :id="inputId"
        v-model="model"
        :type="type"
        :placeholder="placeholder"
        :disabled="disabled"
        :aria-invalid="!!error || undefined"
        :aria-describedby="describedBy(inputId, { error, hint })"
        :class="[
          fieldClass,
          borderClass(error),
          icon && 'pl-7',
          mono && 'font-mono',
        ]"
        spellcheck="false"
        autocomplete="off"
      />
    </div>
  </FormField>
</template>

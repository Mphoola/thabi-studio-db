<script setup>
import { ref, useId } from "vue";
import { Eye, EyeOff } from "lucide-vue-next";
import { borderClass, describedBy, fieldClass } from "./fieldClasses.js";
import FormField from "./FormField.vue";

defineProps({
  label: { type: String, default: "" },
  /** Accessible name when there's no visible label. */
  ariaLabel: { type: String, default: "" },
  hint: { type: String, default: "" },
  error: { type: String, default: "" },
  disabled: { type: Boolean, default: false },
});

const model = defineModel({ type: String, default: "" });
const visible = ref(false);
const inputId = useId();
</script>

<template>
  <FormField :id="inputId" :label="label" :hint="hint" :error="error">
    <div class="relative">
      <input
        :id="inputId"
        v-model="model"
        :type="visible ? 'text' : 'password'"
        :disabled="disabled"
        :aria-label="ariaLabel || undefined"
        :aria-invalid="!!error || undefined"
        :aria-describedby="describedBy(inputId, { error, hint })"
        autocomplete="new-password"
        :class="[fieldClass, borderClass(error), 'pr-8']"
      />
      <button
        type="button"
        class="absolute top-1/2 right-1.5 flex size-5 -translate-y-1/2 items-center justify-center rounded text-fg-subtle hover:bg-surface-hover hover:text-fg focus-visible:ring-3 focus-visible:ring-focus focus-visible:outline-none"
        :aria-label="visible ? 'Hide password' : 'Show password'"
        :aria-pressed="visible"
        :disabled="disabled"
        @click="visible = !visible"
      >
        <component
          :is="visible ? EyeOff : Eye"
          :size="14"
          :stroke-width="1.75"
          aria-hidden="true"
        />
      </button>
    </div>
  </FormField>
</template>

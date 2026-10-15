<script setup>
import { useId } from "vue";
import { ChevronDown } from "lucide-vue-next";
import FormField from "./FormField.vue";
import { borderClass, describedBy, fieldClass } from "./fieldClasses.js";

/** Native <select>, styled to match TextInput. Keeps the platform's keyboard behaviour. */
const props = defineProps({
  label: { type: String, default: "" },
  hint: { type: String, default: "" },
  error: { type: String, default: "" },
  /** @type {{ value: string, label: string, disabled?: boolean }[]} */
  options: { type: Array, required: true },
  disabled: { type: Boolean, default: false },
  id: { type: String, default: "" },
});

const model = defineModel({ type: [String, Number], default: "" });
const autoId = useId();
const selectId = props.id || autoId;
</script>

<template>
  <FormField :id="selectId" :label="label" :hint="hint" :error="error">
    <div class="relative">
      <select
        :id="selectId"
        v-model="model"
        :disabled="disabled"
        :aria-invalid="!!error || undefined"
        :aria-describedby="describedBy(selectId, { error, hint })"
        :class="[fieldClass, borderClass(error)]"
        class="appearance-none pr-7"
      >
        <option
          v-for="option in options"
          :key="option.value"
          :value="option.value"
          :disabled="option.disabled"
        >
          {{ option.label }}
        </option>
      </select>
      <ChevronDown
        :size="14"
        :stroke-width="1.75"
        class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-fg-subtle"
        aria-hidden="true"
      />
    </div>
  </FormField>
</template>

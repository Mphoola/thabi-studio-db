<script setup>
import { computed, useId } from "vue";
import { Check } from "lucide-vue-next";
import { ENVIRONMENTS } from "../../../lib/environments.js";
import { SAFE_MODES, safeModeFor } from "../../../lib/sqlSafety.js";
import FormRow from "../../common/FormRow.vue";
import SelectInput from "../../common/SelectInput.vue";
import TextInput from "../../common/TextInput.vue";

/** Name, colour, environment tag and safe mode: who this connection is. */
const form = defineModel("form", { type: Object, required: true });
defineProps({ fields: { type: Object, required: true } });
const emit = defineEmits(["safe-mode-changed"]);
const nameId = useId();

const environments = ENVIRONMENTS.map(({ value, label }) => ({ value, label }));
const safeModes = SAFE_MODES.map(({ value, label }) => ({ value, label }));
const safeMode = computed({
  get: () => form.value.safeMode,
  set: (value) => {
    form.value.safeMode = value;
    emit("safe-mode-changed");
  },
});
// Whole class names so Tailwind keeps them; "" follows the environment.
const swatches = [
  { value: "", label: "Environment colour", class: "" },
  { value: "red", label: "Red", class: "bg-tag-red" },
  { value: "orange", label: "Orange", class: "bg-tag-orange" },
  { value: "yellow", label: "Yellow", class: "bg-tag-yellow" },
  { value: "green", label: "Green", class: "bg-tag-green" },
  { value: "teal", label: "Teal", class: "bg-tag-teal" },
  { value: "blue", label: "Blue", class: "bg-tag-blue" },
  { value: "purple", label: "Purple", class: "bg-tag-purple" },
  { value: "gray", label: "Gray", class: "bg-tag-gray" },
];
const environmentDot = computed(
  () => ENVIRONMENTS.find((e) => e.value === form.value.environment)?.dotClass,
);
</script>

<template>
  <section class="flex flex-col gap-2.5 rounded-lg bg-surface-hover/60 p-4">
    <FormRow label="Name" :for="nameId" :error="fields.name">
      <TextInput
        :id="nameId"
        v-model="form.name"
        placeholder="Orders (local)"
      />
    </FormRow>
    <FormRow label="Colour" :error="fields.colour">
      <div
        class="flex flex-wrap items-center gap-1.5"
        role="radiogroup"
        aria-label="Colour"
      >
        <button
          v-for="swatch in swatches"
          :key="swatch.value"
          v-tooltip="swatch.label"
          type="button"
          role="radio"
          :aria-checked="form.colour === swatch.value"
          :aria-label="swatch.label"
          class="flex size-6 items-center justify-center rounded-md border border-line-strong focus-visible:ring-2 focus-visible:ring-focus"
          :class="swatch.class || environmentDot"
          @click="form.colour = swatch.value"
        >
          <Check
            v-if="form.colour === swatch.value"
            :size="14"
            :stroke-width="2.5"
            class="text-accent-fg"
            aria-hidden="true"
          />
        </button>
      </div>
    </FormRow>
    <FormRow label="Tag" :error="fields.environment">
      <SelectInput
        v-model="form.environment"
        class="w-48"
        :options="environments"
        aria-label="Environment tag"
      />
    </FormRow>
    <FormRow
      label="Safe mode"
      :error="fields.safeMode"
      :hint="safeModeFor(form.safeMode).hint"
    >
      <SelectInput
        v-model="safeMode"
        :options="safeModes"
        aria-label="Safe mode"
      />
    </FormRow>
  </section>
</template>

<script setup>
import { computed } from "vue";
import { words } from "../../../lib/platform.js";
import CheckboxInput from "../../common/CheckboxInput.vue";
import PasswordInput from "../../common/PasswordInput.vue";
import SelectInput from "../../common/SelectInput.vue";

/**
 * A password or passphrase with where it lives: saved in the OS credential
 * store, or asked for each time (then nothing is saved and the field hides).
 */
const value = defineModel({ type: String, default: "" });
const ask = defineModel("ask", { type: Boolean, default: false });
const clear = defineModel("clear", { type: Boolean, default: false });
defineProps({
  label: { type: String, required: true },
  hasSaved: { type: Boolean, default: false },
  error: { type: String, default: "" },
  placeholder: { type: String, default: "" },
});

const storage = computed({
  get: () => (ask.value ? "ask" : "store"),
  set: (choice) => (ask.value = choice === "ask"),
});
const choices = [
  { value: "store", label: `Store in ${words.credentialStore}` },
  { value: "ask", label: "Ask every time" },
];
</script>

<template>
  <div class="grid grid-cols-[minmax(0,1fr)_minmax(0,13rem)] gap-2">
    <PasswordInput
      v-if="!ask"
      v-model="value"
      :aria-label="label"
      :error="error"
    />
    <p v-else class="h-7 truncate pt-1 text-sm text-fg-subtle">
      Asked for when connecting; nothing is saved.
    </p>
    <SelectInput
      v-model="storage"
      :options="choices"
      :aria-label="`Where the ${label.toLowerCase()} is kept`"
    />
    <p
      v-if="!ask && hasSaved && !value && !clear"
      class="col-span-2 text-xs text-fg-subtle"
    >
      Saved. Leave blank to keep it.
    </p>
    <CheckboxInput
      v-if="!ask && hasSaved"
      v-model="clear"
      class="col-span-2"
      :label="`Remove the saved ${label.toLowerCase()}`"
    />
  </div>
</template>

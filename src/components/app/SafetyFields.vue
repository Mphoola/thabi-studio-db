<script setup>
import { computed } from "vue";
import { SAFE_MODES, safeModeFor } from "../../lib/sqlSafety.js";
import SelectInput from "../common/SelectInput.vue";

/**
 * The safe mode a connection runs with, with a plain explanation of the chosen
 * mode. It sits above the database and SSH settings because it is a decision
 * about the whole connection, not about how it is reached.
 */
const form = defineModel("form", { type: Object, required: true });
defineProps({
  fields: { type: Object, default: () => ({}) },
});
const emit = defineEmits(["changed"]);

const options = SAFE_MODES.map(({ value, label }) => ({ value, label }));
const hint = computed(() => safeModeFor(form.value.safeMode).hint);

function onChange(value) {
  form.value.safeMode = value;
  emit("changed");
}
</script>

<template>
  <section class="flex flex-col gap-2">
    <div>
      <h3 class="text-sm font-semibold text-fg">Safety</h3>
      <p class="text-sm text-fg-muted">
        How much Thabi Studio asks before it sends anything to this connection.
        Set it per connection; Production starts on Confirm Writes.
      </p>
    </div>
    <SelectInput
      :model-value="form.safeMode"
      :options="options"
      label="Safe Mode"
      :error="fields.safeMode"
      :hint="hint"
      @update:model-value="onChange"
    />
  </section>
</template>

<script setup>
import { computed } from "vue";
import { Check, Circle } from "lucide-vue-next";
import { passwordRules } from "../../../lib/appPassword.js";
import PasswordInput from "../../common/PasswordInput.vue";

/** A new app password, its live rules checklist and a confirmation. */
defineProps({
  label: { type: String, default: "New password" },
  error: { type: String, default: "" },
});
const password = defineModel({ type: String, default: "" });
const repeat = defineModel("repeat", { type: String, default: "" });
const rules = computed(() => passwordRules(password.value));
</script>

<template>
  <div class="flex flex-col gap-3 text-left">
    <PasswordInput v-model="password" :label="label" :error="error" />
    <ul class="flex flex-col gap-1 text-sm" aria-label="Password rules">
      <li
        v-for="rule in rules"
        :key="rule.id"
        class="flex items-center gap-1.5"
        :class="rule.met ? 'text-success-text' : 'text-fg-subtle'"
      >
        <component
          :is="rule.met ? Check : Circle"
          :size="12"
          :stroke-width="1.75"
          aria-hidden="true"
        />
        {{ rule.label }}
        <span class="sr-only">{{ rule.met ? "(met)" : "(not met)" }}</span>
      </li>
    </ul>
    <PasswordInput
      v-model="repeat"
      label="Confirm password"
      :error="repeat && repeat !== password ? 'The passwords don’t match.' : ''"
    />
  </div>
</template>

<script setup>
import { useId } from "vue";
import { Check } from "lucide-vue-next";

defineProps({
  label: { type: String, required: true },
  description: { type: String, default: "" },
  disabled: { type: Boolean, default: false },
});

const model = defineModel({ type: Boolean, default: false });
const id = useId();
</script>

<template>
  <div class="flex items-start gap-2">
    <span class="relative mt-px flex size-4 shrink-0">
      <input
        :id="id"
        v-model="model"
        type="checkbox"
        :disabled="disabled"
        :aria-describedby="description ? `${id}-desc` : undefined"
        class="peer size-4 cursor-default appearance-none rounded-sm border border-line-strong bg-surface transition-colors checked:border-accent checked:bg-accent disabled:opacity-50"
      />
      <Check
        :size="12"
        :stroke-width="3"
        class="pointer-events-none absolute top-0.5 left-0.5 hidden text-accent-fg peer-checked:block"
        aria-hidden="true"
      />
    </span>
    <div class="flex flex-col">
      <label :for="id" class="text-base text-fg">{{ label }}</label>
      <p v-if="description" :id="`${id}-desc`" class="text-sm text-fg-subtle">
        {{ description }}
      </p>
    </div>
  </div>
</template>

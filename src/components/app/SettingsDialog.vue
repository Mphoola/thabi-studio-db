<script setup>
import { computed } from "vue";
import { Monitor, Moon, Sun } from "lucide-vue-next";
import { useTheme } from "../../composables/useTheme.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import SegmentedControl from "../common/SegmentedControl.vue";

const open = defineModel("open", { type: Boolean, default: false });
const theme = useTheme();

const appearance = computed({
  get: () => theme.mode.value,
  set: (value) => theme.setMode(value),
});

const options = [
  { value: "system", label: "System", icon: Monitor },
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
];
</script>

<template>
  <BaseDialog v-model:open="open" title="Settings">
    <div class="flex items-center justify-between gap-4">
      <div class="flex flex-col">
        <span id="appearance-label" class="text-base text-fg">Appearance</span>
        <span class="text-sm text-fg-subtle">System follows your Mac.</span>
      </div>
      <SegmentedControl
        v-model="appearance"
        :options="options"
        label="Appearance"
      />
    </div>
    <template #footer>
      <BaseButton variant="primary" @click="open = false">Done</BaseButton>
    </template>
  </BaseDialog>
</template>

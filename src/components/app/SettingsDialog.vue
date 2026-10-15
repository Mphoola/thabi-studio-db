<script setup>
import { computed } from "vue";
import { Monitor, Moon, Sun } from "lucide-vue-next";
import { useAppSettings } from "../../composables/useAppSettings.js";
import { useTheme } from "../../composables/useTheme.js";
import SelectInput from "../common/SelectInput.vue";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import SegmentedControl from "../common/SegmentedControl.vue";

const open = defineModel("open", { type: Boolean, default: false });
const theme = useTheme();

const appearance = computed({
  get: () => theme.mode.value,
  set: (value) => theme.setMode(value),
});

const { settings, persistAppSetting } = useAppSettings();
const queryTimeout = computed({
  get: () => String(settings.value.queryTimeoutSeconds ?? 0),
  set: (value) => persistAppSetting("queryTimeoutSeconds", Number(value)),
});
const maxRows = computed({
  get: () => String(settings.value.maxResultRows ?? 10000),
  set: (value) => persistAppSetting("maxResultRows", Number(value)),
});
const rowLimits = [
  { value: "1000", label: "1,000 rows" },
  { value: "10000", label: "10,000 rows" },
  { value: "100000", label: "100,000 rows" },
  { value: "1000000", label: "1,000,000 rows" },
];

const timeouts = [
  { value: "0", label: "No limit" },
  { value: "30", label: "30 seconds" },
  { value: "60", label: "1 minute" },
  { value: "300", label: "5 minutes" },
  { value: "900", label: "15 minutes" },
];

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
    <div
      class="mt-4 flex items-start justify-between gap-4 border-t border-line pt-4"
    >
      <div class="flex flex-col">
        <span class="text-base text-fg">Query timeout</span>
        <span class="max-w-64 text-sm text-fg-subtle">
          Queries running longer are stopped. SQL Server stops them by resetting
          the tab’s session.
        </span>
      </div>
      <SelectInput v-model="queryTimeout" class="w-36" :options="timeouts" />
    </div>
    <div
      class="mt-4 flex items-start justify-between gap-4 border-t border-line pt-4"
    >
      <div class="flex flex-col">
        <span class="text-base text-fg">Rows kept per result</span>
        <span class="max-w-64 text-sm text-fg-subtle">
          Results are fetched up to this many rows and kept on this Mac for
          scrolling, sorting, copying and export.
        </span>
      </div>
      <SelectInput v-model="maxRows" class="w-36" :options="rowLimits" />
    </div>
    <template #footer>
      <BaseButton variant="primary" @click="open = false">Done</BaseButton>
    </template>
  </BaseDialog>
</template>

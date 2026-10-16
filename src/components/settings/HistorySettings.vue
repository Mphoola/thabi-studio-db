<script setup>
import { computed } from "vue";
import { confirm } from "../../composables/useConfirm.js";
import { useAppSettings } from "../../composables/useAppSettings.js";
import { useLibrary } from "../../composables/useLibrary.js";
import { useToasts } from "../../composables/useToasts.js";
import { words } from "../../lib/platform.js";
import BaseButton from "../common/BaseButton.vue";
import SelectInput from "../common/SelectInput.vue";
import SwitchInput from "../common/SwitchInput.vue";
import SettingRow from "./SettingRow.vue";
import SettingsGroup from "./SettingsGroup.vue";

const library = useLibrary();
const toasts = useToasts();
const { settings, persistAppSetting } = useAppSettings();

const historyEnabled = computed({
  get: () => settings.value.historyEnabled,
  set: (value) => persistAppSetting("historyEnabled", value),
});
const retention = computed({
  get: () => String(settings.value.historyRetentionDays),
  set: (value) => persistAppSetting("historyRetentionDays", Number(value)),
});
const retentions = [
  { value: "7", label: "1 week" },
  { value: "30", label: "30 days" },
  { value: "90", label: "90 days" },
  { value: "365", label: "1 year" },
  { value: "0", label: "Until 10,000 queries" },
];

async function clearHistory() {
  const ok = await confirm({
    title: "Clear query history?",
    message: `Every query in history is deleted from ${words.computer}. Saved queries aren't affected.`,
    confirmLabel: "Clear History",
    destructive: true,
  });
  if (!ok) return;
  const count = await library.clearAll();
  toasts.info(
    `Cleared ${count.toLocaleString()} ${count === 1 ? "query" : "queries"}`,
  );
}
</script>

<template>
  <SettingsGroup title="Query history">
    <SettingRow
      title="Keep history"
      :description="`SQL you run is kept on ${words.computer}, including any values typed into it. Turn it off for sensitive work.`"
    >
      <SwitchInput v-model="historyEnabled" label="Keep history" hide-label />
    </SettingRow>
    <SettingRow title="Keep queries for">
      <SelectInput
        v-model="retention"
        class="w-44"
        :options="retentions"
        :disabled="!historyEnabled"
        aria-label="Keep history for"
      />
    </SettingRow>
    <SettingRow
      title="Clear history"
      description="Deletes every query in history now."
    >
      <BaseButton size="sm" @click="clearHistory">Clear History…</BaseButton>
    </SettingRow>
  </SettingsGroup>
</template>

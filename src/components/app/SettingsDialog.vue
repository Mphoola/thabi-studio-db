<script setup>
import { computed } from "vue";
import { Monitor, Moon, Sun } from "lucide-vue-next";
import { confirm } from "../../composables/useConfirm.js";
import { useAppSettings } from "../../composables/useAppSettings.js";
import { useLibrary } from "../../composables/useLibrary.js";
import { useTheme } from "../../composables/useTheme.js";
import { useToasts } from "../../composables/useToasts.js";
import { words } from "../../lib/platform.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import SegmentedControl from "../common/SegmentedControl.vue";
import SelectInput from "../common/SelectInput.vue";
import SwitchInput from "../common/SwitchInput.vue";
import EditorSettings from "./EditorSettings.vue";
import SettingRow from "./SettingRow.vue";

const open = defineModel("open", { type: Boolean, default: false });
const theme = useTheme();
const library = useLibrary();
const toasts = useToasts();
const { settings, persistAppSetting } = useAppSettings();

/** A setting bound to a select (values are strings in the DOM). */
const numberSetting = (key) =>
  computed({
    get: () => String(settings.value[key]),
    set: (value) => persistAppSetting(key, Number(value)),
  });

const appearance = computed({
  get: () => theme.mode.value,
  set: (value) => theme.setMode(value),
});
const fontSize = numberSetting("editorFontSize");
const queryTimeout = numberSetting("queryTimeoutSeconds");
const maxRows = numberSetting("maxResultRows");
const retention = numberSetting("historyRetentionDays");
const historyEnabled = computed({
  get: () => settings.value.historyEnabled,
  set: (value) => persistAppSetting("historyEnabled", value),
});

const themes = [
  { value: "system", label: "System", icon: Monitor },
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
];
const fontSizes = [11, 12, 13, 14, 15, 16, 18].map((n) => ({
  value: String(n),
  label: `${n} pt`,
}));
const timeouts = [
  { value: "0", label: "No limit" },
  { value: "30", label: "30 seconds" },
  { value: "60", label: "1 minute" },
  { value: "300", label: "5 minutes" },
  { value: "900", label: "15 minutes" },
];
const rowLimits = [1000, 10000, 100000, 1000000].map((n) => ({
  value: String(n),
  label: `${n.toLocaleString()} rows`,
}));
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
  <BaseDialog v-model:open="open" title="Settings" size="lg">
    <div class="flex flex-col divide-y divide-line">
      <SettingRow
        title="Appearance"
        :description="`System follows ${words.computer === 'this Mac' ? 'your Mac' : 'your system'}.`"
      >
        <SegmentedControl
          v-model="appearance"
          :options="themes"
          label="Appearance"
        />
      </SettingRow>
      <SettingRow title="Editor font size">
        <SelectInput
          v-model="fontSize"
          class="w-36"
          :options="fontSizes"
          aria-label="Editor font size"
        />
      </SettingRow>
      <EditorSettings />
      <SettingRow
        title="Query timeout"
        description="Longer queries are stopped. SQL Server stops them by resetting the tab’s session."
      >
        <SelectInput
          v-model="queryTimeout"
          class="w-36"
          :options="timeouts"
          aria-label="Query timeout"
        />
      </SettingRow>
      <SettingRow
        title="Rows kept per result"
        :description="`Fetched and kept on ${words.computer} for scrolling, sorting, copying and export.`"
      >
        <SelectInput
          v-model="maxRows"
          class="w-36"
          :options="rowLimits"
          aria-label="Rows kept per result"
        />
      </SettingRow>
      <SettingRow
        title="Query history"
        :description="`SQL you run is kept on ${words.computer}, including any values typed into it. Turn it off for sensitive work.`"
      >
        <div class="flex flex-col items-end gap-2">
          <SwitchInput v-model="historyEnabled" label="Keep history" />
          <SelectInput
            v-model="retention"
            class="w-44"
            :options="retentions"
            :disabled="!historyEnabled"
            aria-label="Keep history for"
          />
          <BaseButton size="sm" variant="ghost" @click="clearHistory"
            >Clear History…</BaseButton
          >
        </div>
      </SettingRow>
    </div>
    <template #footer>
      <BaseButton variant="primary" @click="open = false">Done</BaseButton>
    </template>
  </BaseDialog>
</template>

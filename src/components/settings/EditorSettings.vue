<script setup>
import { computed } from "vue";
import { useAppSettings } from "../../composables/useAppSettings.js";
import { TIME_ZONE_CHOICES } from "../../lib/timeZones.js";
import SelectInput from "../common/SelectInput.vue";
import SwitchInput from "../common/SwitchInput.vue";
import TextInput from "../common/TextInput.vue";
import SettingRow from "./SettingRow.vue";

/** SQL editor and value display preferences (Phases 12 and 13). */
const { settings, persistAppSetting } = useAppSettings();

const bind = (key, toValue = (v) => v) =>
  computed({
    get: () => settings.value[key],
    set: (value) => persistAppSetting(key, toValue(value)),
  });
const fontFamily = bind("editorFontFamily", (v) => v.trim());
const tabSize = computed({
  get: () => String(settings.value.editorTabSize),
  set: (value) => persistAppSetting("editorTabSize", Number(value)),
});
const wordWrap = bind("editorWordWrap");
const keywordCase = bind("editorKeywordCase");
const vim = bind("editorVim");
const timeZone = bind("displayTimeZone");

const tabSizes = [2, 4, 8].map((n) => ({
  value: String(n),
  label: `${n} spaces`,
}));
const cases = [
  { value: "upper", label: "UPPER CASE" },
  { value: "lower", label: "lower case" },
];
</script>

<template>
  <SettingRow
    title="Editor font"
    description="A font installed on this computer, like “JetBrains Mono”. Leave blank for the system monospace font."
  >
    <TextInput
      v-model.lazy="fontFamily"
      class="w-44"
      placeholder="System monospace"
      aria-label="Editor font"
    />
  </SettingRow>
  <SettingRow title="Indentation">
    <SelectInput
      v-model="tabSize"
      class="w-36"
      :options="tabSizes"
      aria-label="Indentation"
    />
  </SettingRow>
  <SettingRow
    title="Keywords"
    description="How autocomplete writes SQL keywords."
  >
    <SelectInput
      v-model="keywordCase"
      class="w-36"
      :options="cases"
      aria-label="Keyword case"
    />
  </SettingRow>
  <SettingRow title="Editor behaviour">
    <div class="flex flex-col items-end gap-2">
      <SwitchInput v-model="wordWrap" label="Wrap long lines" />
      <SwitchInput v-model="vim" label="Vim key bindings" />
    </div>
  </SettingRow>
  <SettingRow
    title="Times with a time zone"
    description="Values that carry an offset (timestamptz, datetimeoffset) are shown in this zone. Times without one are always shown as stored."
  >
    <SelectInput
      v-model="timeZone"
      class="w-56"
      :options="TIME_ZONE_CHOICES"
      aria-label="Time zone for display"
    />
  </SettingRow>
</template>

<script setup>
import { computed } from "vue";
import { useAppSettings } from "../../composables/useAppSettings.js";
import SelectInput from "../common/SelectInput.vue";
import SwitchInput from "../common/SwitchInput.vue";
import TextInput from "../common/TextInput.vue";
import SettingRow from "./SettingRow.vue";
import SettingsGroup from "./SettingsGroup.vue";

/** SQL editor preferences. */
const { settings, persistAppSetting } = useAppSettings();

const bind = (key, toValue = (v) => v) =>
  computed({
    get: () => settings.value[key],
    set: (value) => persistAppSetting(key, toValue(value)),
  });
const numeric = (key) =>
  computed({
    get: () => String(settings.value[key]),
    set: (value) => persistAppSetting(key, Number(value)),
  });
const fontSize = numeric("editorFontSize");
const fontFamily = bind("editorFontFamily", (v) => v.trim());
const tabSize = numeric("editorTabSize");
const wordWrap = bind("editorWordWrap");
const keywordCase = bind("editorKeywordCase");
const vim = bind("editorVim");

const fontSizes = [11, 12, 13, 14, 15, 16, 18].map((n) => ({
  value: String(n),
  label: `${n} pt`,
}));
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
  <SettingsGroup title="Font">
    <SettingRow title="Size">
      <SelectInput
        v-model="fontSize"
        class="w-36"
        :options="fontSizes"
        aria-label="Editor font size"
      />
    </SettingRow>
    <SettingRow
      title="Font family"
      description="A font installed on this computer, like “JetBrains Mono”. Leave blank for the system monospace font."
    >
      <TextInput
        v-model.lazy="fontFamily"
        class="w-44"
        placeholder="System monospace"
        aria-label="Editor font"
      />
    </SettingRow>
  </SettingsGroup>
  <SettingsGroup title="Typing">
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
    <SettingRow title="Wrap long lines">
      <SwitchInput v-model="wordWrap" label="Wrap long lines" hide-label />
    </SettingRow>
    <SettingRow
      title="Vim key bindings"
      description="Normal, insert and visual modes in the SQL editor."
    >
      <SwitchInput v-model="vim" label="Vim key bindings" hide-label />
    </SettingRow>
  </SettingsGroup>
</template>

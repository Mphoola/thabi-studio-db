<script setup>
import { computed } from "vue";
import { Monitor, Moon, Sun } from "lucide-vue-next";
import { useTheme } from "../../composables/useTheme.js";
import { words } from "../../lib/platform.js";
import SegmentedControl from "../common/SegmentedControl.vue";
import SettingRow from "./SettingRow.vue";
import SettingsGroup from "./SettingsGroup.vue";

const theme = useTheme();
const appearance = computed({
  get: () => theme.mode.value,
  set: (value) => theme.setMode(value),
});
const themes = [
  { value: "system", label: "System", icon: Monitor },
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
];
</script>

<template>
  <SettingsGroup title="Appearance">
    <SettingRow
      title="Theme"
      :description="`System follows ${words.computer === 'this Mac' ? 'your Mac' : 'your system'}.`"
    >
      <SegmentedControl v-model="appearance" :options="themes" label="Theme" />
    </SettingRow>
  </SettingsGroup>
</template>

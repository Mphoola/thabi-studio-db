<script setup>
import { computed } from "vue";
import { useAppSettings } from "../../composables/useAppSettings.js";
import { words } from "../../lib/platform.js";
import { TIME_ZONE_CHOICES } from "../../lib/timeZones.js";
import SelectInput from "../common/SelectInput.vue";
import SettingRow from "./SettingRow.vue";
import SettingsGroup from "./SettingsGroup.vue";

/** How queries run and how their results are kept and shown. */
const { settings, persistAppSetting } = useAppSettings();
const numeric = (key) =>
  computed({
    get: () => String(settings.value[key]),
    set: (value) => persistAppSetting(key, Number(value)),
  });
const queryTimeout = numeric("queryTimeoutSeconds");
const maxRows = numeric("maxResultRows");
const timeZone = computed({
  get: () => settings.value.displayTimeZone,
  set: (value) => persistAppSetting("displayTimeZone", value),
});

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
</script>

<template>
  <SettingsGroup
    title="Running queries"
    description="Defaults for every connection. A connection can set its own under More options."
  >
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
  </SettingsGroup>
  <SettingsGroup title="Showing values">
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
  </SettingsGroup>
</template>

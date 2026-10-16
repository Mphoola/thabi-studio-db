<script setup>
import { computed, ref, watch } from "vue";
import { useAppLock } from "../../composables/useAppLock.js";
import { useAppSettings } from "../../composables/useAppSettings.js";
import { AUTO_LOCK_OPTIONS } from "../../lib/appPassword.js";
import { isTauri } from "../../lib/tauri/ipc.js";
import BaseButton from "../common/BaseButton.vue";
import SelectInput from "../common/SelectInput.vue";
import AppPasswordDialog from "../app/AppPasswordDialog.vue";
import SecurityQuestionsDialog from "../app/lock/SecurityQuestionsDialog.vue";
import SettingRow from "./SettingRow.vue";
import SettingsGroup from "./SettingsGroup.vue";

const lock = useAppLock();
const { settings, persistAppSetting } = useAppSettings();
const enabled = computed(() => lock.state.value.enabled);
const dialogMode = ref("set");
const dialogOpen = ref(false);
const questionsOpen = ref(false);
const removingQuestions = ref(false);
const recovery = ref(null);

async function loadRecovery() {
  recovery.value = enabled.value
    ? await lock.recoveryOptions().catch(() => null)
    : null;
}
watch(enabled, loadRecovery, { immediate: true });

const autoLock = computed({
  get: () => String(settings.value.autoLockMinutes),
  set: (value) => persistAppSetting("autoLockMinutes", Number(value)),
});
const codesLeft = computed(() => recovery.value?.codesLeft ?? 0);
const hasQuestions = computed(() => recovery.value?.questions.length > 0);

function openDialog(mode) {
  dialogMode.value = mode;
  dialogOpen.value = true;
}

function openQuestions(remove) {
  removingQuestions.value = remove;
  questionsOpen.value = true;
}
</script>

<template>
  <SettingsGroup
    title="App lock"
    description="An extra lock in front of every connection, for when others can reach this computer."
  >
    <SettingRow
      title="App password"
      :description="
        enabled
          ? 'Asked for when Thabi Studio opens and after it has been idle. Nothing, including saved passwords, can be used until it is entered.'
          : 'Adds a lock in front of every connection. Use at least 10 characters with upper- and lowercase letters.'
      "
    >
      <div class="flex flex-col items-end gap-2">
        <template v-if="enabled">
          <div class="flex gap-1.5">
            <BaseButton size="sm" @click="openDialog('change')"
              >Change…</BaseButton
            >
            <BaseButton size="sm" variant="ghost" @click="openDialog('remove')"
              >Turn Off…</BaseButton
            >
          </div>
          <BaseButton size="sm" variant="ghost" @click="lock.lock()"
            >Lock Now</BaseButton
          >
        </template>
        <BaseButton
          v-else
          size="sm"
          :disabled="!isTauri()"
          @click="openDialog('set')"
          >Set Password…</BaseButton
        >
      </div>
    </SettingRow>
    <SettingRow
      v-if="enabled"
      title="Lock when idle"
      description="Locks every window when none has been used for this long."
    >
      <SelectInput
        v-model="autoLock"
        class="w-44"
        :options="AUTO_LOCK_OPTIONS"
        aria-label="Lock when idle"
      />
    </SettingRow>
  </SettingsGroup>
  <SettingsGroup
    v-if="enabled"
    title="Recovery"
    description="Ways to set a new password if you forget this one."
  >
    <SettingRow
      title="Recovery codes"
      :description="
        codesLeft
          ? `${codesLeft} of 10 unused. Each one sets a new password once if you forget it.`
          : 'None left. Make new ones so you can still get back in.'
      "
    >
      <BaseButton size="sm" @click="openDialog('codes')"
        >Make New Codes…</BaseButton
      >
    </SettingRow>
    <SettingRow
      title="Security questions"
      :description="
        hasQuestions
          ? 'Set. Answering all three sets a new password if you forget it.'
          : 'Optional. Three questions you answer to set a new password if you forget it.'
      "
    >
      <div class="flex gap-1.5">
        <BaseButton size="sm" @click="openQuestions(false)">{{
          hasQuestions ? "Change…" : "Set Up…"
        }}</BaseButton>
        <BaseButton
          v-if="hasQuestions"
          size="sm"
          variant="ghost"
          @click="openQuestions(true)"
          >Remove…</BaseButton
        >
      </div>
    </SettingRow>
  </SettingsGroup>
  <AppPasswordDialog
    v-model:open="dialogOpen"
    :mode="dialogMode"
    @changed="loadRecovery"
  />
  <SecurityQuestionsDialog
    v-model:open="questionsOpen"
    :remove="removingQuestions"
    @changed="loadRecovery"
  />
</template>

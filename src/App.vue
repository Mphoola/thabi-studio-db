<script setup>
import { ref, watchEffect } from "vue";
import { useAppLock } from "./composables/useAppLock.js";
import LockScreen from "./components/app/LockScreen.vue";
import AppShell from "./components/app/AppShell.vue";
import ErrorBoundary from "./components/app/ErrorBoundary.vue";
import ConfirmHost from "./components/common/ConfirmHost.vue";
import ParameterPromptHost from "./components/common/ParameterPromptHost.vue";
import SecretPromptHost from "./components/common/SecretPromptHost.vue";
import ToastHost from "./components/common/ToastHost.vue";
import TooltipHost from "./components/common/TooltipHost.vue";

// The workspace mounts only after the first unlock, so nothing loads behind
// the lock at startup. A later lock covers it instead, keeping open tabs.
const lock = useAppLock();
const started = ref(false);
watchEffect(() => {
  if (lock.state.value.loaded && !lock.state.value.locked) started.value = true;
});
</script>

<template>
  <ErrorBoundary>
    <AppShell v-if="started" :inert="lock.state.value.locked || undefined" />
  </ErrorBoundary>
  <ConfirmHost />
  <SecretPromptHost />
  <ParameterPromptHost />
  <ToastHost />
  <TooltipHost />
  <LockScreen v-if="lock.state.value.locked" />
</template>

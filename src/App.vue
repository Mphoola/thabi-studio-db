<script setup>
import AppShell from "./components/app/AppShell.vue";
import ErrorBoundary from "./components/app/ErrorBoundary.vue";
import ConfirmHost from "./components/common/ConfirmHost.vue";
import SecretPromptHost from "./components/common/SecretPromptHost.vue";
import ToastHost from "./components/common/ToastHost.vue";
import TooltipHost from "./components/common/TooltipHost.vue";
</script>

<template>
  <ErrorBoundary>
    <AppShell />
  </ErrorBoundary>
  <ConfirmHost />
  <SecretPromptHost />
  <ToastHost />
  <TooltipHost />
</template>

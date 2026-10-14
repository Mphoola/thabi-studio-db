<script setup>
import AppShell from "./components/app/AppShell.vue";
import ErrorBoundary from "./components/app/ErrorBoundary.vue";
import ConfirmHost from "./components/common/ConfirmHost.vue";
import ToastHost from "./components/common/ToastHost.vue";
</script>

<template>
  <ErrorBoundary>
    <AppShell />
  </ErrorBoundary>
  <ConfirmHost />
  <ToastHost />
</template>

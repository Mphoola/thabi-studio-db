<script setup>
import { ref } from "vue";
import AboutDialog from "./AboutDialog.vue";
import AppSidebar from "./AppSidebar.vue";
import StatusBar from "./StatusBar.vue";
import WelcomePane from "./WelcomePane.vue";

const aboutOpen = ref(false);
</script>

<template>
  <div class="flex h-screen flex-col bg-canvas text-fg">
    <div class="flex min-h-0 flex-1">
      <AppSidebar @open-about="aboutOpen = true" />
      <main class="min-w-0 flex-1 overflow-auto">
        <WelcomePane />
      </main>
    </div>
    <StatusBar />
    <AboutDialog v-model:open="aboutOpen" />
  </div>
</template>

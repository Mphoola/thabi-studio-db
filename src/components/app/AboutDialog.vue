<script setup>
import { useAppInfo } from "../../composables/useAppInfo.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";

const open = defineModel("open", { type: Boolean, default: false });
const { info } = useAppInfo();
</script>

<template>
  <BaseDialog
    v-model:open="open"
    size="sm"
    title="Thabi Studio"
    :description="info ? `Version ${info.version}` : undefined"
  >
    <p class="text-fg-muted">
      Thabi Studio connects directly from this Mac to your database servers. It
      has no user accounts, no telemetry and no cloud backend.
    </p>
    <template #footer>
      <BaseButton variant="primary" autofocus @click="open = false">
        Done
      </BaseButton>
    </template>
  </BaseDialog>
</template>

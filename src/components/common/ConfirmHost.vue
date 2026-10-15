<script setup>
import { computed } from "vue";
import { useConfirmState } from "../../composables/useConfirm.js";
import BaseButton from "./BaseButton.vue";
import BaseDialog from "./BaseDialog.vue";

const { request, settle } = useConfirmState();

const open = computed({
  get: () => request.value !== null,
  set: (value) => {
    if (!value) settle(false);
  },
});
</script>

<template>
  <BaseDialog
    v-if="request"
    v-model:open="open"
    size="sm"
    :title="request.title"
    :description="request.message"
  >
    <p
      v-if="request.detail"
      class="selectable rounded-md bg-surface-hover px-3 py-2 font-mono text-sm break-all text-fg"
    >
      {{ request.detail }}
    </p>
    <template #footer>
      <BaseButton :autofocus="request.destructive" @click="settle(false)">
        {{ request.cancelLabel ?? "Cancel" }}
      </BaseButton>
      <BaseButton
        :variant="request.destructive ? 'danger' : 'primary'"
        :autofocus="!request.destructive"
        @click="settle(true)"
      >
        {{ request.confirmLabel ?? "OK" }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

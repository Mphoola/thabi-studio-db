<script setup>
import { computed, ref, watch } from "vue";
import { useConfirmState } from "../../composables/useConfirm.js";
import BaseButton from "./BaseButton.vue";
import BaseDialog from "./BaseDialog.vue";
import TextInput from "./TextInput.vue";

const { request, settle } = useConfirmState();
/** What the user has typed when the confirmation needs a typed answer. */
const typed = ref("");

const open = computed({
  get: () => request.value !== null,
  set: (value) => {
    if (!value) settle(false);
  },
});

watch(request, () => (typed.value = ""));

/** `requireText` asks for a value to be typed before Confirm is allowed. */
const required = computed(() => request.value?.requireText || null);
const typedMatches = computed(
  () => typed.value.trim() === (required.value ?? ""),
);
const canConfirm = computed(() => !required.value || typedMatches.value);

function confirmNow() {
  if (canConfirm.value) settle(true);
}
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
    <TextInput
      v-if="required"
      v-model="typed"
      class="mt-3"
      mono
      :label="`Type ${required} to confirm`"
      :placeholder="required"
      autocomplete="off"
    />
    <template #footer>
      <BaseButton :autofocus="request.destructive" @click="settle(false)">
        {{ request.cancelLabel ?? "Cancel" }}
      </BaseButton>
      <BaseButton
        :variant="request.destructive ? 'danger' : 'primary'"
        :autofocus="!request.destructive"
        :disabled="!canConfirm"
        @click="confirmNow"
      >
        {{ request.confirmLabel ?? "OK" }}
      </BaseButton>
    </template>
  </BaseDialog>
</template>

<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { Lock } from "lucide-vue-next";
import logoUrl from "../../assets/logo.svg";
import { useAppLock } from "../../composables/useAppLock.js";
import { toUserMessage } from "../../lib/errors.js";
import BaseButton from "../common/BaseButton.vue";
import InlineAlert from "../common/InlineAlert.vue";
import PasswordInput from "../common/PasswordInput.vue";
import LockRecovery from "./lock/LockRecovery.vue";

/**
 * Covers every window while the app password is needed. It is a modal
 * <dialog> opened last, so it sits above any dialog that was already open,
 * and Escape can't close it. Rust refuses everything else meanwhile.
 */
const lock = useAppLock();
const dialog = ref(null);
const password = ref("");
const error = ref("");
const busy = ref(false);
const resetting = ref(false);
const waitUntil = ref(0);
const now = ref(Date.now());
let ticker = null;

const waitSeconds = computed(() =>
  Math.max(0, Math.ceil((waitUntil.value - now.value) / 1000)),
);

function startWait(seconds) {
  if (!seconds) return;
  waitUntil.value = Date.now() + seconds * 1000;
  now.value = Date.now();
}

async function focusPassword() {
  await nextTick();
  dialog.value?.querySelector("input")?.focus();
}

onMounted(() => {
  dialog.value.showModal();
  startWait(lock.state.value.retryAfterSeconds);
  ticker = setInterval(() => (now.value = Date.now()), 500);
  focusPassword();
});
onBeforeUnmount(() => clearInterval(ticker));

async function unlock() {
  if (!password.value || busy.value || waitSeconds.value) return;
  busy.value = true;
  error.value = "";
  try {
    await lock.unlock(password.value);
  } catch (problem) {
    error.value =
      problem?.fields?.password ?? toUserMessage(problem ?? undefined);
    startWait(problem?.data?.retryAfterSeconds);
    password.value = "";
    focusPassword();
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <dialog
    ref="dialog"
    class="m-0 h-screen max-h-none w-screen max-w-none bg-canvas p-0 text-fg backdrop:bg-canvas"
    aria-labelledby="lock-title"
    @cancel.prevent
  >
    <div
      data-tauri-drag-region
      class="flex h-full flex-col items-center justify-center px-4"
    >
      <form
        v-if="!resetting"
        class="flex w-full max-w-xs flex-col items-center gap-4 text-center"
        @submit.prevent="unlock"
      >
        <img :src="logoUrl" alt="" class="size-16" />
        <div class="flex flex-col gap-1">
          <h1
            id="lock-title"
            class="flex items-center justify-center gap-1.5 text-lg font-semibold"
          >
            <Lock :size="16" :stroke-width="1.75" aria-hidden="true" />
            Thabi Studio is locked
          </h1>
          <p class="text-sm text-fg-muted">
            Enter the app password to get back to your connections.
          </p>
        </div>

        <InlineAlert
          v-if="lock.state.value.error"
          tone="danger"
          title="The lock couldn't be read"
          :message="lock.state.value.error"
          class="w-full text-left"
        />
        <template v-else>
          <PasswordInput
            v-model="password"
            class="w-full text-left"
            aria-label="App password"
            :error="error"
            :disabled="busy || waitSeconds > 0"
          />
          <p v-if="waitSeconds" class="text-sm text-warning-text" role="status">
            Try again in {{ waitSeconds }} s.
          </p>
        </template>

        <BaseButton
          v-if="lock.state.value.error"
          variant="primary"
          class="w-full justify-center"
          @click="lock.refresh()"
          >Try Again</BaseButton
        >
        <BaseButton
          v-else
          type="submit"
          variant="primary"
          class="w-full justify-center"
          :loading="busy"
          :disabled="!password || waitSeconds > 0"
          >Unlock</BaseButton
        >
        <button
          type="button"
          class="text-sm text-fg-subtle hover:text-fg-muted hover:underline"
          @click="resetting = true"
        >
          Forgot password?
        </button>
      </form>
      <LockRecovery v-else @back="((resetting = false), focusPassword())" />
    </div>
  </dialog>
</template>

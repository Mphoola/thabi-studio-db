import { readonly, ref } from "vue";
import {
  appLockStatus,
  lockApp,
  recoverWithAnswers,
  recoverWithCode,
  regenerateRecoveryCodes,
  recoveryOptions,
  removeAppPassword,
  removeSecurityQuestions,
  setSecurityQuestions,
  resetAppPassword,
  setAppPassword,
  unlockApp,
} from "../lib/tauri/commands.js";
import { isTauri, setUnlockWaiter } from "../lib/tauri/ipc.js";
import { toUserMessage } from "../lib/errors.js";
import { useAppSettings } from "./useAppSettings.js";
import { pauseCommandsWhen } from "./useCommands.js";

/**
 * The app password lock. Rust owns it and refuses other commands while it is
 * locked; this mirrors its state for the lock screen, and locks again after
 * the idle time chosen in Settings. See docs/decisions/0033-app-password.md.
 */

const state = ref({
  loaded: false,
  enabled: false,
  locked: true,
  retryAfterSeconds: 0,
  error: "",
});
let waiters = [];

// Activity is shared through localStorage so a second window in use keeps
// the first one from locking everything.
const ACTIVITY_KEY = "thabi.lastActivity";
const CHECK_EVERY_MS = 15_000;
let lastActivity = Date.now();
let lastShared = 0;

function apply(status) {
  state.value = { ...status, loaded: true, error: "" };
  if (!status.locked) {
    const ready = waiters;
    waiters = [];
    ready.forEach((resolve) => resolve());
  }
}

function noteActivity() {
  lastActivity = Date.now();
  if (lastActivity - lastShared < 5_000) return;
  lastShared = lastActivity;
  try {
    localStorage.setItem(ACTIVITY_KEY, String(lastActivity));
  } catch {
    // Private storage off: this window's own activity still counts.
  }
}

function latestActivity() {
  try {
    return Math.max(lastActivity, Number(localStorage.getItem(ACTIVITY_KEY)));
  } catch {
    return lastActivity;
  }
}

function watchIdle() {
  const { settings } = useAppSettings();
  for (const type of ["pointerdown", "pointermove", "keydown", "wheel"])
    window.addEventListener(type, noteActivity, {
      capture: true,
      passive: true,
    });
  setInterval(() => {
    const minutes = settings.value.autoLockMinutes;
    const { enabled, locked } = state.value;
    if (!enabled || locked || !minutes) return;
    if (Date.now() - latestActivity() >= minutes * 60_000) lock();
  }, CHECK_EVERY_MS);
}

/** Reads the lock before the app mounts. Call once, from main.js. */
export async function initAppLock() {
  if (!isTauri()) {
    apply({ enabled: false, locked: false, retryAfterSeconds: 0 });
    return;
  }
  pauseCommandsWhen(() => state.value.locked);
  setUnlockWaiter(() =>
    state.value.locked
      ? new Promise((resolve) => waiters.push(resolve))
      : Promise.resolve(),
  );
  try {
    const { listen } = await import("@tauri-apps/api/event");
    await listen("app-lock-changed", (event) => apply(event.payload));
  } catch {
    // No event bridge (the mock preview): the command results still apply.
  }
  await refresh();
  watchIdle();
}

async function refresh() {
  try {
    apply(await appLockStatus());
  } catch (error) {
    // Fail closed: stay locked and say why, with a way to try again.
    state.value = { ...state.value, error: toUserMessage(error) };
  }
}

async function lock() {
  apply(await lockApp());
}

export function useAppLock() {
  return {
    state: readonly(state),
    refresh,
    lock,
    async unlock(password) {
      apply(await unlockApp(password));
      noteActivity();
    },
    /** Resolves to new recovery codes when the password was first set. */
    async setPassword(current, password) {
      const result = await setAppPassword(current || null, password);
      apply(result.status);
      return result.recoveryCodes ?? null;
    },
    recoveryOptions,
    regenerateCodes: regenerateRecoveryCodes,
    setQuestions: setSecurityQuestions,
    removeQuestions: removeSecurityQuestions,
    async recoverWithCode(code, password) {
      apply(await recoverWithCode(code, password));
      noteActivity();
    },
    async recoverWithAnswers(answers, password) {
      apply(await recoverWithAnswers(answers, password));
      noteActivity();
    },
    async removePassword(current) {
      apply(await removeAppPassword(current));
    },
    async reset() {
      apply(await resetAppPassword());
    },
  };
}

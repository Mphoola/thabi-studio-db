import { ref } from "vue";
import { confirm } from "./useConfirm.js";
import { useConnectionProfiles } from "./useConnectionProfiles.js";
import { useHostKeyTrust } from "./useHostKeyTrust.js";
import { useToasts } from "./useToasts.js";
import { toUserDetail, toUserMessage } from "../lib/errors.js";

const busy = ref(new Set());

/** Connect, disconnect, duplicate and delete, with consistent messages. */
export function useConnectionActions() {
  const store = useConnectionProfiles();
  const hostKeys = useHostKeyTrust();
  const toasts = useToasts();

  function markBusy(id, value) {
    const next = new Set(busy.value);
    if (value) next.add(id);
    else next.delete(id);
    busy.value = next;
  }

  /** Selects the profile and connects it (no-op if already connected). */
  async function connect(profile) {
    markBusy(profile.id, true);
    try {
      await store.select(profile.id);
      if (!store.isConnected(profile.id))
        await hostKeys.run(() => store.connect(profile.id));
      return true;
    } catch (error) {
      toasts.error(
        `Couldn’t connect to “${profile.name}”`,
        toUserDetail(error),
      );
      return false;
    } finally {
      markBusy(profile.id, false);
    }
  }

  async function disconnect(profile) {
    markBusy(profile.id, true);
    try {
      await store.disconnect(profile.id);
    } catch (error) {
      toasts.error(
        `Couldn’t disconnect from “${profile.name}”`,
        toUserDetail(error),
      );
    } finally {
      markBusy(profile.id, false);
    }
  }

  function toggle(profile) {
    return store.isConnected(profile.id)
      ? disconnect(profile)
      : connect(profile);
  }

  async function duplicate(profile) {
    try {
      const warning = await store.duplicate(profile.id);
      if (warning)
        toasts.error("Duplicated, but Keychain needs attention", warning);
    } catch (error) {
      toasts.error(
        `Couldn’t duplicate “${profile.name}”`,
        toUserMessage(error),
      );
    }
  }

  async function remove(profile) {
    const ok = await confirm({
      title: `Delete “${profile.name}”?`,
      message:
        "Its saved password is also removed from your keychain. This can't be undone.",
      confirmLabel: "Delete",
      destructive: true,
    });
    if (!ok) return;
    try {
      await store.remove(profile.id);
    } catch (error) {
      toasts.error(`Couldn’t delete “${profile.name}”`, toUserMessage(error));
    }
  }

  return {
    busy,
    connect,
    disconnect,
    toggle,
    duplicate,
    remove,
    edit: (profile) => store.openEdit(profile.id),
  };
}

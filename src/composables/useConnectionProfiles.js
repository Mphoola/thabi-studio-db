import { computed, readonly, ref } from "vue";
import {
  connectConnectionProfile,
  createConnectionProfile,
  deleteConnectionProfile,
  disconnectConnectionProfile,
  duplicateConnectionProfile,
  listConnections,
  listConnectionProfiles,
  reorderConnectionProfiles,
  selectConnectionProfile,
  testConnectionInput,
  updateConnectionProfile,
} from "../lib/tauri/commands.js";
import { sortProfiles, upsertProfile } from "../lib/connectionGroups.js";
import { isTauri } from "../lib/tauri/ipc.js";
import { applyLocalAppSetting, useAppSettings } from "./useAppSettings.js";
import { useDialogs } from "./useDialogs.js";
import { askForRequiredSecret } from "./useSecretPrompt.js";

const profiles = ref([]);
const loaded = ref(false);
const editingId = ref(null);
/** Live connection status per profile id, as reported by Rust. */
const statuses = ref(new Map());
let loadPromise;

async function refreshStatuses() {
  const list = await listConnections();
  statuses.value = new Map(list.map((status) => [status.profileId, status]));
}

function setStatus(profileId, status) {
  const next = new Map(statuses.value);
  if (status) next.set(profileId, status);
  else next.delete(profileId);
  statuses.value = next;
}

export function useConnectionProfiles() {
  const { settings } = useAppSettings();
  const dialogs = useDialogs();
  const selectedProfile = computed(
    () =>
      profiles.value.find(
        (profile) => profile.id === settings.value.selectedProfileId,
      ) ?? null,
  );

  async function load() {
    if (loaded.value) return profiles.value;
    if (loadPromise) return loadPromise;
    loadPromise = (async () => {
      if (isTauri()) profiles.value = await listConnectionProfiles();
      if (isTauri()) await refreshStatuses();
      loaded.value = true;
      return profiles.value;
    })();
    try {
      return await loadPromise;
    } finally {
      loadPromise = null;
    }
  }

  /** Re-reads the saved connections, after an import for example. */
  async function refresh() {
    if (isTauri()) profiles.value = await listConnectionProfiles();
  }

  function openNew() {
    editingId.value = null;
    dialogs.show("connection");
  }

  function openEdit(id) {
    if (!profiles.value.some((profile) => profile.id === id)) return;
    editingId.value = id;
    dialogs.show("connection");
  }

  async function save(input) {
    const result = editingId.value
      ? await updateConnectionProfile(editingId.value, input)
      : await createConnectionProfile(input);
    // A new connection is added; an edited one replaces its old copy.
    profiles.value = upsertProfile(profiles.value, result.profile);
    // Rust closes the old connection when a profile's settings change.
    if (editingId.value) setStatus(result.profile.id, null);
    dialogs.close();
    return { warning: result.warning, profile: result.profile };
  }

  async function duplicate(id) {
    const result = await duplicateConnectionProfile(id);
    profiles.value = sortProfiles([...profiles.value, result.profile]);
    return result.warning;
  }

  /**
   * Remembers a drag: `ids` is the group's new order. Rust returns the list
   * in its canonical order, which is what the page then shows.
   */
  async function reorder(ids) {
    profiles.value = await reorderConnectionProfiles(ids);
  }

  async function select(id) {
    await selectConnectionProfile(id);
    applyLocalAppSetting("selectedProfileId", id);
  }

  async function remove(id) {
    await deleteConnectionProfile(id);
    setStatus(id, null);
    profiles.value = profiles.value.filter((profile) => profile.id !== id);
    if (settings.value.selectedProfileId === id) {
      applyLocalAppSetting("selectedProfileId", null);
    }
  }

  /** Tests form values. Resolves to `{ server, elapsedMs }`. */
  function test(input, profileId = null) {
    return testConnectionInput(input, profileId);
  }

  /**
   * Opens a connection. A profile set to "Ask every time" keeps no secret, so
   * Rust says which one it needs; the user types it and the attempt is
   * retried. The value is used for this connection only and never stored.
   */
  async function connect(id, database = null, attempt = 0, secrets = {}) {
    setStatus(id, { profileId: id, state: "connecting", server: null });
    try {
      setStatus(id, await connectConnectionProfile(id, database, secrets));
      // Rust records the time too; mirror it so "last connected" updates now.
      const now = new Date().toISOString();
      profiles.value = profiles.value.map((p) =>
        p.id === id ? { ...p, lastConnectedAt: now } : p,
      );
    } catch (error) {
      setStatus(id, null);
      // A profile may need two secrets (database and SSH); ask for each in turn.
      if (error?.kind === "secret_required" && error.data && attempt < 3) {
        const value = await askForRequiredSecret(error.data);
        if (value === null) throw error;
        return connect(id, database, attempt + 1, {
          ...secrets,
          [error.data.secret]: value,
        });
      }
      throw error;
    }
  }

  async function disconnect(id) {
    try {
      await disconnectConnectionProfile(id);
    } finally {
      setStatus(id, null);
    }
  }

  function statusFor(id) {
    return statuses.value.get(id) ?? null;
  }

  function isConnected(id) {
    return statusFor(id)?.state === "connected";
  }

  return {
    profiles: readonly(profiles),
    loaded: readonly(loaded),
    editingId: readonly(editingId),
    selectedProfile,
    isConnected,
    statusFor,
    forgetStatus: (id) => setStatus(id, null),
    nativeAvailable: isTauri(),
    load,
    refresh,
    /** Re-reads live connection states, e.g. when another window changed them. */
    syncStatuses: () => (isTauri() ? refreshStatuses() : Promise.resolve()),
    openNew,
    openEdit,
    save,
    duplicate,
    reorder,
    select,
    remove,
    test,
    connect,
    disconnect,
  };
}

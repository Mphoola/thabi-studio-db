import { computed, readonly, ref } from "vue";
import {
  createConnectionProfile,
  deleteConnectionProfile,
  duplicateConnectionProfile,
  listConnectionProfiles,
  selectConnectionProfile,
  updateConnectionProfile,
} from "../lib/tauri/commands.js";
import { isTauri } from "../lib/tauri/ipc.js";
import { applyLocalAppSetting, useAppSettings } from "./useAppSettings.js";
import { useDialogs } from "./useDialogs.js";

const profiles = ref([]);
const loaded = ref(false);
const editingId = ref(null);
let loadPromise;

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
      loaded.value = true;
      return profiles.value;
    })();
    try {
      return await loadPromise;
    } finally {
      loadPromise = null;
    }
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
    const index = profiles.value.findIndex(
      (profile) => profile.id === result.profile.id,
    );
    profiles.value =
      index < 0
        ? [...profiles.value, result.profile].sort((a, b) =>
            a.name.localeCompare(b.name),
          )
        : profiles.value.map((profile) =>
            profile.id === result.profile.id ? result.profile : profile,
          );
    dialogs.close();
    return result.warning;
  }

  async function duplicate(id) {
    const result = await duplicateConnectionProfile(id);
    profiles.value = [...profiles.value, result.profile].sort((a, b) =>
      a.name.localeCompare(b.name),
    );
    return result.warning;
  }

  async function select(id) {
    await selectConnectionProfile(id);
    applyLocalAppSetting("selectedProfileId", id);
  }

  async function remove(id) {
    await deleteConnectionProfile(id);
    profiles.value = profiles.value.filter((profile) => profile.id !== id);
    if (settings.value.selectedProfileId === id) {
      applyLocalAppSetting("selectedProfileId", null);
    }
  }

  return {
    profiles: readonly(profiles),
    loaded: readonly(loaded),
    editingId: readonly(editingId),
    selectedProfile,
    nativeAvailable: isTauri(),
    load,
    openNew,
    openEdit,
    save,
    duplicate,
    select,
    remove,
  };
}

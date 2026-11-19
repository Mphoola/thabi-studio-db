import { ref } from "vue";

/**
 * Which import is open. Module state: the explorer's menu opens it and the
 * panel renders it.
 */
const target = ref(null);
const open = ref(false);
const restoreTarget = ref(null);
const restoreOpen = ref(false);
const backupTarget = ref(null);
const backupOpen = ref(false);

export function useImportDialog() {
  function show({ profileId, database, schema = null, table = null }) {
    target.value = { profileId, database, schema, table };
    open.value = true;
  }
  /** Running a SQL dump into a database. */
  function showRestore({ profileId, database }) {
    restoreTarget.value = { profileId, database };
    restoreOpen.value = true;
  }
  /** A backup or restore with the engine's own tools. */
  function showBackup({ profileId, database, mode }) {
    backupTarget.value = { profileId, database, mode };
    backupOpen.value = true;
  }
  return {
    target,
    open,
    show,
    restoreTarget,
    restoreOpen,
    showRestore,
    backupTarget,
    backupOpen,
    showBackup,
  };
}

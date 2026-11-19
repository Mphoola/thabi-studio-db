import { ref } from "vue";

/**
 * Which database or schema dialog is open. Module state, like the backup
 * dialog's: the explorer's menu opens it and the panel renders it.
 */
const target = ref(null);
const open = ref(false);

export function useContainerDialog() {
  /** @param {"createDatabase"|"dropDatabase"|"createSchema"|"dropSchema"} kind */
  function show(kind, { profileId, database = null, name = null }) {
    target.value = { kind, profileId, database, name };
    open.value = true;
  }
  return { target, open, show };
}

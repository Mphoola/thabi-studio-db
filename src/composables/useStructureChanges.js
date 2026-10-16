import { computed, ref } from "vue";
import { describeEdit, stageChange, toPlan } from "../lib/structurePlan.js";
import { applyTableDdl, previewTableDdl } from "../lib/tauri/commands.js";
import { toUserMessage } from "../lib/errors.js";
import { useToasts } from "./useToasts.js";

/**
 * The staged structure changes for one table tab, and the two commands that
 * show and run them.
 *
 * Changes live on the tab, so switching tabs and coming back does not throw
 * away what was being changed. Nothing is sent until the SQL has been shown and
 * agreed to.
 */
export function useStructureChanges({
  tab,
  structure,
  target,
  profileId,
  onApplied,
}) {
  const toasts = useToasts();
  const reviewOpen = ref(false);
  const preview = ref(null);
  const previewing = ref(false);
  const applying = ref(false);
  const error = ref("");

  if (!tab.structureEdits) tab.structureEdits = [];
  const edits = computed(() => tab.structureEdits);
  const plan = computed(() => toPlan(structure.value, edits.value));
  const dirty = computed(() => edits.value.length > 0);

  function stage(change) {
    tab.structureEdits = stageChange(edits.value, change);
    error.value = "";
  }

  function undo(change) {
    tab.structureEdits = edits.value.filter((one) => one !== change);
  }

  function discard() {
    tab.structureEdits = [];
    preview.value = null;
    error.value = "";
  }

  /** The SQL for what is staged, and the dialog to show it in. */
  async function review() {
    if (!dirty.value) return;
    previewing.value = true;
    error.value = "";
    try {
      preview.value = await previewTableDdl(
        profileId.value,
        tab.id,
        { ...target.value, schema: target.value.schema || null },
        plan.value,
      );
      reviewOpen.value = true;
    } catch (failure) {
      error.value = toUserMessage(failure);
    } finally {
      previewing.value = false;
    }
  }

  /** Runs the script that was shown, then clears what was staged. */
  async function run() {
    applying.value = true;
    try {
      await applyTableDdl(
        profileId.value,
        tab.id,
        { ...target.value, schema: target.value.schema || null },
        plan.value,
      );
      toasts.success("Changes applied");
      reviewOpen.value = false;
      discard();
      onApplied?.();
    } catch (failure) {
      toasts.error("Couldn't apply the changes", toUserMessage(failure));
    } finally {
      applying.value = false;
    }
  }

  return {
    edits,
    plan,
    dirty,
    reviewOpen,
    preview,
    previewing,
    applying,
    error,
    stage,
    undo,
    discard,
    review,
    run,
    describeEdit,
  };
}

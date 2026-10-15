import { computed, ref, watch } from "vue";
import { confirm } from "../../composables/useConfirm.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useRowEdits } from "../../composables/useRowEdits.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail } from "../../lib/errors.js";
import { applyConfirmationFor } from "../../lib/sqlSafety.js";
import {
  applyRowChanges,
  describeObject,
  lookupReference,
  previewRowChanges,
} from "../../lib/tauri/commands.js";

const plural = (n, one, many = `${one}s`) =>
  `${n.toLocaleString()} ${n === 1 ? one : many}`;

/**
 * Inline editing for a table tab: finds the primary key, stages edits
 * (useRowEdits), previews the SQL and applies it in one transaction.
 * `columns` are the loaded result's columns; `reload` re-reads the rows.
 */
export function useTableEditing({ tab, node, profileId, columns, reload }) {
  const store = useConnectionProfiles();
  const toasts = useToasts();
  const primaryKey = ref([]);
  const columnInfo = ref(new Map()); // name -> structure column
  const keyChecked = ref(false);
  const applying = ref(false);
  const preview = ref(null); // statements shown in the review dialog

  const editor = useRowEdits(tab, columns, primaryKey);
  const isTable = computed(() => node.value?.kind === "table");
  const profile = computed(
    () => store.profiles.value.find((p) => p.id === profileId.value) ?? null,
  );
  /** A read-only connection can't be edited through the app. */
  const readOnlyMode = computed(() => profile.value?.safeMode === "read-only");
  /** Why rows can't be edited, or "" when they can. */
  const readOnlyReason = computed(() => {
    if (!isTable.value) return "Read-only: views can't be edited here";
    if (readOnlyMode.value)
      return "Read-only: this connection's safe mode is Read-only";
    if (!keyChecked.value) return "";
    return editor.enabled.value
      ? ""
      : "Read-only: this table has no primary key";
  });

  /** Resolves once the key for the current node has been read (or failed). */
  let keyLoad = Promise.resolve();

  async function loadKey() {
    keyChecked.value = false;
    primaryKey.value = [];
    columnInfo.value = new Map();
    const target = node.value;
    if (!isTable.value || !profileId.value) return;
    try {
      const structure = await describeObject(profileId.value, target);
      if (node.value !== target) return;
      columnInfo.value = new Map(structure.columns.map((c) => [c.name, c]));
      primaryKey.value = structure.columns
        .filter((c) => c.primaryKey)
        .sort((a, b) => a.primaryKey - b.primaryKey)
        .map((c) => c.name);
    } catch {
      primaryKey.value = []; // Stays read-only; the data view shows errors.
    } finally {
      keyChecked.value = true;
    }
  }
  watch(
    node,
    () => {
      keyLoad = loadKey();
    },
    { immediate: true },
  );

  /** How a column is edited: a list of enum values, a foreign key lookup, or free text (null). */
  editor.pickerFor = (col) => {
    const info = columnInfo.value.get(columns.value[col]?.name);
    if (info?.enumValues?.length)
      return {
        kind: "enum",
        nullable: info.nullable,
        search: async (text) =>
          info.enumValues
            .filter((v) => v.toLowerCase().includes(text.trim().toLowerCase()))
            .map((value) => ({ value, label: null })),
      };
    if (info?.references)
      return {
        kind: "reference",
        nullable: info.nullable,
        table: info.references.table,
        search: (text) =>
          lookupReference(
            profileId.value,
            tab.id,
            node.value.database,
            info.references,
            text,
          ),
      };
    return null;
  };

  const target = () => ({
    database: node.value.database,
    schema: node.value.schema ?? "",
    name: node.value.name,
  });

  async function showPreview() {
    try {
      preview.value = await previewRowChanges(
        profileId.value,
        tab.id,
        target(),
        editor.changes(),
      );
    } catch (error) {
      toasts.error("Couldn’t prepare the changes", toUserDetail(error));
    }
  }

  async function confirmApply() {
    // Read-only connections never get as far as asking: nothing is sent.
    if (readOnlyMode.value) return false;
    const summary = editor.summary.value;
    const question = applyConfirmationFor(summary, {
      safeMode: profile.value?.safeMode,
      environment: profile.value?.environment,
      connection: profile.value?.name ?? "this connection",
      table: node.value.name,
    });
    return question ? confirm(question) : true;
  }

  async function apply() {
    const changes = editor.changes();
    if (!changes.length || applying.value || !(await confirmApply())) return;
    applying.value = true;
    try {
      const count = await applyRowChanges(
        profileId.value,
        tab.id,
        target(),
        changes,
      );
      preview.value = null;
      editor.discard();
      toasts.success(
        `Saved ${plural(count, "change")} to “${node.value.name}”`,
      );
      reload();
    } catch (error) {
      toasts.error("Nothing was saved", toUserDetail(error));
    } finally {
      applying.value = false;
    }
  }

  async function discard() {
    const { total } = editor.summary.value;
    if (
      total > 1 &&
      !(await confirm({
        title: `Discard ${plural(total, "change")}?`,
        message: "Your staged edits are thrown away. The table isn't changed.",
        confirmLabel: "Discard",
        destructive: true,
      }))
    )
      return;
    editor.discard();
  }

  /** Rows can be edited: a table with a key, on a connection that allows it. */
  const editable = computed(() => !readOnlyMode.value && editor.enabled.value);

  return {
    editor,
    editable,
    /** The table's key column names, in key order. Paging needs them. */
    primaryKey,
    /**
     * Waits for the key to be known. Paging a table without it would fetch
     * the first page unordered and the rest ordered, so the two could show
     * the same row twice and skip another.
     */
    keyReady: () => keyLoad,
    readOnlyReason,
    applying,
    preview,
    showPreview,
    apply,
    discard,
  };
}

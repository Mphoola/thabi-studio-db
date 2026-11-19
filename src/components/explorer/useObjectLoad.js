import { computed, ref, watch } from "vue";
import { describeObject, openObjectData } from "../../lib/tauri/commands.js";

/**
 * Reads what an object tab shows: a page of data or the structure, one
 * result per view, kept so switching views doesn't read again. A result that
 * arrives after the tab moved to another object is dropped.
 *
 * The data request is built late, through callbacks, because the filters
 * and the primary key it needs are set up after this, from the columns it
 * reads.
 */
export function useObjectLoad({
  tab,
  node,
  view,
  profileId,
  columns,
  pageSize,
  filterRequest,
  primaryKey,
  onData,
}) {
  const results = ref({}); // view -> { status, value?, error? }

  async function load(which = view.value) {
    const target = node.value;
    const id = profileId.value;
    if (!target || !id) return;
    results.value = { ...results.value, [which]: { status: "loading" } };
    const request = which === "data" ? filterRequest() : [];
    try {
      const value =
        which === "data"
          ? await openObjectData(id, tab.id, {
              ...target,
              // One page at a time, ordered by the key (Phase 12), sized by
              // the connection's row limit.
              maxRows: pageSize.value,
              filters: request,
              // The key must be known first: paging without it would order
              // the first page differently from the rest.
              primaryKey: await primaryKey(),
            })
          : await describeObject(id, target);
      if (node.value !== target) return;
      results.value = {
        ...results.value,
        [which]: { status: "loaded", value },
      };
      if (which === "data") {
        columns.value = value.columns;
        onData(request.length);
      }
    } catch (error) {
      if (node.value === target)
        results.value = {
          ...results.value,
          [which]: { status: "error", error },
        };
    }
  }

  watch(node, () => {
    results.value = {};
    columns.value = [];
    load();
  });
  watch(view, (value) => !results.value[value] && load(value));
  // A foreign key jump re-filters an open tab and asks for a reload.
  watch(
    () => tab.reloadToken,
    () => load("data"),
  );

  const current = computed(
    () => results.value[view.value] ?? { status: "idle" },
  );
  /** The page just read, and the estimate of how many rows the table has. */
  const page = computed(() =>
    current.value.status === "loaded" ? current.value.value : null,
  );

  return { load, current, page };
}

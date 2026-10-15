import { ref, shallowRef, watch } from "vue";

export const PAGE_SIZE = 200;

/**
 * Loads grid rows a page at a time from a data source and caches them.
 * Source: { columns, total, firstRows, load(offset, limit, sort) }.
 * Only pages near what's on screen are ever requested.
 */
export function useGridData(source) {
  const pages = shallowRef(new Map());
  const sort = ref(null); // { column, descending } | null
  const version = ref(0); // bumps when pages change, for re-rendering
  const error = ref(null);
  let inflight = new Map();
  let generation = 0;

  function reset() {
    generation += 1;
    inflight = new Map();
    const map = new Map();
    if (!sort.value && source.value.firstRows?.length) {
      // The first response may hold more than one page.
      for (let i = 0; i * PAGE_SIZE < source.value.firstRows.length; i += 1)
        map.set(
          i,
          source.value.firstRows.slice(i * PAGE_SIZE, (i + 1) * PAGE_SIZE),
        );
    }
    pages.value = map;
    error.value = null;
    version.value += 1;
  }

  async function loadPage(index) {
    if (pages.value.has(index) || inflight.has(index)) return;
    const mine = generation;
    const request = source.value
      .load(index * PAGE_SIZE, PAGE_SIZE, sort.value)
      .then((rows) => {
        if (mine !== generation) return;
        const next = new Map(pages.value);
        next.set(index, rows);
        pages.value = next;
        version.value += 1;
      })
      .catch((e) => {
        if (mine === generation) error.value = e;
      })
      .finally(() => inflight.delete(index));
    inflight.set(index, request);
  }

  /** Row at a display position, or null while its page loads. */
  function rowAt(position) {
    const index = Math.floor(position / PAGE_SIZE);
    const page = pages.value.get(index);
    if (!page) {
      loadPage(index);
      return null;
    }
    return page[position - index * PAGE_SIZE] ?? null;
  }

  function toggleSort(column) {
    const current = sort.value;
    if (!current || current.column !== column)
      sort.value = { column, descending: false };
    else if (!current.descending) sort.value = { column, descending: true };
    else sort.value = null;
  }

  watch(sort, reset);
  watch(source, () => {
    sort.value = null;
    reset();
  });
  reset();

  /** Every row loaded so far, for measuring columns. */
  function loadedRows() {
    const rows = [];
    for (const page of pages.value.values()) rows.push(...page);
    return rows;
  }

  return { rowAt, loadedRows, sort, toggleSort, version, error };
}

import { computed, ref, shallowRef, watch } from "vue";

export const PAGE_SIZE = 200;

/**
 * Loads grid rows a page at a time from a data source and caches them.
 * Source: { columns, total, firstRows, load(offset, limit, sort), pageSize? }.
 * Only pages near what's on screen are ever requested.
 *
 * The page size is the source's: a table's is the connection's row limit, a
 * query result's is `PAGE_SIZE`. Everything that converts a row position to an
 * offset reads it from one place, since a page that was half the size it was
 * asked to be would quietly show the wrong rows.
 */
export function useGridData(source) {
  const pages = shallowRef(new Map());
  // Page the loaded window opens at. Scrolling moves within the window; a jump
  // moves the window itself, so the grid never holds more rows than the user
  // has actually looked at.
  const windowStart = ref(0);
  const sort = ref(null); // { column, descending } | null
  const version = ref(0); // bumps when pages change, for re-rendering
  const error = ref(null);
  let inflight = new Map();
  let generation = 0;

  /** Rows in one page for the current source. */
  function pageSize() {
    return source.value.pageSize ?? PAGE_SIZE;
  }

  function reset() {
    generation += 1;
    inflight = new Map();
    const map = new Map();
    const size = pageSize();
    if (!sort.value && source.value.firstRows?.length) {
      // The first response may hold more than one page.
      for (let i = 0; i * size < source.value.firstRows.length; i += 1)
        map.set(i, source.value.firstRows.slice(i * size, (i + 1) * size));
    }
    pages.value = map;
    windowStart.value = 0;
    error.value = null;
    version.value += 1;
  }

  async function loadPage(index) {
    if (pages.value.has(index) || inflight.has(index)) return;
    const mine = generation;
    const size = pageSize();
    const request = source.value
      .load(index * size, size, sort.value)
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
    const size = pageSize();
    const index = Math.floor(position / size);
    const page = pages.value.get(index);
    if (!page) {
      loadPage(index);
      return null;
    }
    return page[position - index * size] ?? null;
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

  /** Loads the page at a position, so the pager can jump ahead of the scroll. */
  function ensureAt(position) {
    loadPage(Math.floor(position / pageSize()));
  }

  /** Rows loaded locally, which is what the scroller is tall enough to show. */
  const loaded = computed(() => {
    void version.value;
    let last = 0;
    for (const index of pages.value.keys()) last = Math.max(last, index + 1);
    return Math.min(last * pageSize(), source.value.total);
  });

  /**
   * Whether the table has rows past the window.
   *
   * A short page is the end of the table: the server had nothing more to give,
   * so there is no page after it however large the catalog estimate is.
   */
  const hasMore = computed(() => {
    const end = loaded.value;
    if (end >= source.value.total) return false;
    const lastIndex = Math.ceil(end / pageSize()) - 1;
    return (pages.value.get(lastIndex)?.length ?? 0) === pageSize();
  });

  /** Scrolls forward by one page, which is how scrolling keeps going. */
  function extend() {
    loadPage(Math.ceil(loaded.value / pageSize()));
  }

  /**
   * Moves the window to a page, dropping what was loaded.
   *
   * Jumping to page 500 of a million-row table must not fetch the half million
   * rows before it, so the window is what moves, not the scrollbar.
   */
  async function goto(index) {
    const size = pageSize();
    const total = source.value.total;
    const last = Math.max(0, Math.ceil(total / size) - 1);
    const target = Math.min(Math.max(index, 0), last);
    generation += 1;
    inflight = new Map();
    pages.value = new Map();
    windowStart.value = target;
    error.value = null;
    version.value += 1;
    await loadPage(target);
  }

  return {
    rowAt,
    loadedRows,
    sort,
    toggleSort,
    version,
    error,
    pageSize,
    ensureAt,
    loaded,
    hasMore,
    extend,
    goto,
    windowStart,
  };
}

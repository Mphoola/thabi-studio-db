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
  /** Pages asked for but not yet finished, so none is asked for twice. */
  const requested = new Set();
  let queue = Promise.resolve();
  let generation = 0;

  /** Rows in one page for the current source. */
  function pageSize() {
    return source.value.pageSize ?? PAGE_SIZE;
  }

  function reset() {
    generation += 1;
    inflight = new Map();
    requested.clear();
    queue = Promise.resolve();
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

  /**
   * Loads one page, one request at a time.
   *
   * A tab runs one statement at a time, and the server says so: two pages
   * asked for together come back as "a query is already running on this tab".
   * Scrolling and a page jump can both ask, so requests queue here rather than
   * racing each other.
   */
  function loadPage(index) {
    if (pages.value.has(index)) return Promise.resolve();
    if (inflight.has(index)) return inflight.get(index);
    const mine = generation;
    const size = pageSize();
    requested.add(index);
    // Returned, not just queued: a page jump has to wait for its rows before it
    // scrolls, or the pager moves first and the grid catches up afterwards.
    const request = queue
      .then(() => source.value.load(index * size, size, sort.value))
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
      .finally(() => {
        requested.delete(index);
        inflight.delete(index);
      });
    queue = request;
    inflight.set(index, request);
    return request;
  }

  /** Highest page index loaded, which is where the window ends. */
  function lastLoaded() {
    let last = windowStart.value;
    for (const index of pages.value.keys()) last = Math.max(last, index);
    return last;
  }

  /**
   * Row at a display position, or null while its page loads.
   *
   * A position is relative to the window, not to the table: after a jump to
   * page 500 the top row of the screen is page 500's first row, so reading
   * position 0 as page 0 would show the table's opening rows under a header
   * that says otherwise.
   */
  function rowAt(position) {
    const size = pageSize();
    const inWindow = Math.floor(position / size);
    const index = windowStart.value + inWindow;
    const page = pages.value.get(index);
    if (!page) {
      loadPage(index);
      return null;
    }
    return page[position - inWindow * size] ?? null;
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

  /** Rows in the window, which is what the scroller is tall enough to show. */
  const loaded = computed(() => {
    void version.value;
    const size = pageSize();
    const end = Math.min((lastLoaded() + 1) * size, source.value.total);
    return Math.max(0, end - windowStart.value * size);
  });

  /**
   * Whether the table has rows past the window.
   *
   * A short page is the end of the table: the server had nothing more to give,
   * so there is no page after it however large the catalog estimate is.
   */
  const hasMore = computed(() => {
    void version.value;
    const size = pageSize();
    const last = lastLoaded();
    if ((last + 1) * size >= source.value.total) return false;
    return (pages.value.get(last)?.length ?? 0) === size;
  });

  /** Scrolls forward by one page, which is how scrolling keeps going. */
  function extend() {
    return loadPage(lastLoaded() + 1);
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
    return loadPage(target);
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

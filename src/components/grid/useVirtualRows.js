import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

/**
 * The rows the results grid renders: only those near the viewport, with
 * paging as the end comes into view and jumps between pages.
 */
export function useVirtualRows({
  data,
  rowCount,
  scroller,
  rowHeight,
  overscan,
}) {
  const ROW = rowHeight;
  const OVERSCAN = overscan;
  const scrollTop = ref(0);
  const viewport = ref(400);
  let observer;
  onMounted(() => {
    observer = new ResizeObserver(
      ([entry]) => (viewport.value = entry.contentRect.height),
    );
    observer.observe(scroller.value);
  });
  onBeforeUnmount(() => observer?.disconnect());

  const first = computed(() =>
    Math.max(0, Math.floor(scrollTop.value / ROW) - OVERSCAN),
  );
  const last = computed(() =>
    Math.min(
      rowCount.value,
      Math.ceil((scrollTop.value + viewport.value) / ROW) + OVERSCAN,
    ),
  );
  /** Which page is on screen, for the pager. */
  const page = computed(
    () =>
      data.windowStart.value +
      Math.floor(scrollTop.value / (ROW * data.pageSize())),
  );

  /** Keeps scrolling going: the next page loads as the end comes into view. */
  watch(
    () => [scrollTop.value, viewport.value],
    () => {
      if (!data.hasMore.value) return;
      const end = rowCount.value * ROW;
      if (scrollTop.value + viewport.value >= end - ROW * 20) data.extend();
    },
  );

  /** Jumps to a page; the window moves there rather than fetching up to it. */
  async function goToPage(index) {
    await data.goto(index);
    scrollTop.value = 0;
    if (scroller.value) scroller.value.scrollTop = 0;
  }

  const visible = computed(() => {
    void data.version.value;
    const rows = [];
    for (let i = first.value; i < last.value; i += 1)
      rows.push({ index: i, cells: data.rowAt(i) });
    return rows;
  });

  return { scrollTop, viewport, first, last, page, goToPage, visible };
}

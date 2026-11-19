import { computed, ref } from "vue";
import { tableSource } from "../../lib/gridSources.js";
import { pagerProps } from "../../lib/tablePaging.js";

/**
 * The grid source for a table's data view and the pager under it. The
 * source reads further pages from Rust as the grid scrolls; nothing beyond
 * the page on screen is held in the webview.
 */
export function useTablePager({
  tab,
  view,
  node,
  page,
  columns,
  pageSize,
  profileId,
  filterRequest,
  primaryKey,
}) {
  const previewSource = computed(() =>
    view.value === "data" && page.value
      ? tableSource({
          profileId: profileId.value,
          tabId: tab.id,
          target: node.value,
          filters: filterRequest(),
          primaryKey: primaryKey(),
          estimate: page.value.estimatedTotal ?? null,
          // The live list, not a snapshot: the grid and the editor must agree.
          columns,
          pageSize: pageSize.value,
          firstRows: page.value.rows,
        })
      : null,
  );
  const gridView = ref(null);

  /** What the pager shows; see lib/tablePaging.js. */
  const pager = computed(() =>
    pagerProps(
      gridView.value?.paging ?? null,
      previewSource.value,
      pageSize.value,
    ),
  );
  const goToPage = (index) => gridView.value?.goToPage(index);

  return { previewSource, gridView, pager, goToPage };
}

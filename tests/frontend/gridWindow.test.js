import { describe, expect, it } from "vitest";
import { nextTick, shallowRef } from "vue";
import { useGridData, PAGE_SIZE } from "../../src/composables/useGridData.js";
import { UNKNOWN_TOTAL } from "../../src/lib/tablePaging.js";

/**
 * A table with a million rows cannot be scrolled through as one long strip:
 * the spacer would be twenty-six million pixels, and holding the rows behind it
 * is the thing paging exists to avoid. The grid instead shows a window of loaded
 * pages that grows as the user scrolls and moves wholesale on a jump.
 */
function makeSource({ total = 1_100_000, pageSize = 1_000 } = {}) {
  const calls = [];
  const row = (offset) =>
    Array.from({ length: pageSize }, (_, i) => ({
      kind: "value",
      value: String(offset + i),
    }));
  return {
    calls,
    source: shallowRef({
      columns: [{ name: "id", typeName: "INT" }],
      total,
      estimated: true,
      firstRows: row(0),
      pageSize,
      load(offset, limit, sort) {
        calls.push({ offset, limit, sort });
        // A short page is the end of the table, as the server would answer.
        return Promise.resolve(
          offset + limit >= total
            ? row(offset).slice(0, total - offset)
            : row(offset),
        );
      },
    }),
  };
}

describe("the grid holds a window of a large table", () => {
  it("asks for the source's page size, not the query default", async () => {
    const { source, calls } = makeSource();
    const data = useGridData(source);
    expect(data.pageSize()).toBe(1_000);
    // The first page came with the source, so it is not asked for again.
    data.ensureAt(0);
    await nextTick();
    expect(calls).toHaveLength(0);
    // The next one is, and in units of the source's page.
    data.ensureAt(1_000);
    await nextTick();
    expect(calls[0]).toMatchObject({ offset: 1_000, limit: 1_000 });
  });

  it("falls back to the query page size when a source has none", () => {
    const { source } = makeSource();
    source.value = { ...source.value, pageSize: undefined };
    expect(useGridData(source).pageSize()).toBe(PAGE_SIZE);
  });

  it("starts with one page loaded and no more", () => {
    const { source } = makeSource();
    const data = useGridData(source);
    expect(data.loaded.value).toBe(1_000);
    expect(data.hasMore.value).toBe(true);
    expect(data.windowStart.value).toBe(0);
  });

  it("grows the window one page at a time", async () => {
    const { source } = makeSource();
    const data = useGridData(source);
    await data.extend();
    expect(data.loaded.value).toBe(2_000);
    await data.extend();
    expect(data.loaded.value).toBe(3_000);
  });

  it("knows the end from a short page, not from the estimate", async () => {
    const { source } = makeSource({ total: 1_500 });
    const data = useGridData(source);
    await data.extend();
    expect(data.loaded.value).toBe(1_500);
    // The estimate said more rows existed; the server's short page is the truth.
    expect(data.hasMore.value).toBe(false);
  });

  it("jumps without fetching the pages before it", async () => {
    const { source, calls } = makeSource();
    const data = useGridData(source);
    await data.goto(500);
    expect(data.windowStart.value).toBe(500);
    expect(data.loaded.value).toBe(501_000);
    // One request, for the page jumped to.
    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({ offset: 500_000, limit: 1_000 });
  });

  it("does not jump past the end", async () => {
    const { source, calls } = makeSource({ total: 1_500 });
    const data = useGridData(source);
    await data.goto(99);
    expect(calls[0]).toMatchObject({ offset: 1_000, limit: 1_000 });
    expect(data.windowStart.value).toBe(1);
  });

  it("reads the row a position means, in the right page", async () => {
    const { source } = makeSource();
    const data = useGridData(source);
    await data.extend();
    expect(data.rowAt(0)?.value).toBe("0");
    expect(data.rowAt(999)?.value).toBe("999");
    expect(data.rowAt(1_000)?.value).toBe("1000");
  });

  it("returns to the first page when the source changes", async () => {
    const { source } = makeSource();
    const data = useGridData(source);
    await data.goto(500);
    source.value = { ...source.value, total: 1_100_000 };
    await nextTick();
    expect(data.windowStart.value).toBe(0);
    expect(data.loaded.value).toBe(1_000);
  });

  it("loads nothing beyond a table whose size is unknown", async () => {
    const { source, calls } = makeSource({ total: UNKNOWN_TOTAL });
    const data = useGridData(source);
    // No estimate means no "last page", but scrolling still moves forward.
    expect(data.hasMore.value).toBe(true);
    await data.extend();
    expect(calls[0]).toMatchObject({ offset: 1_000, limit: 1_000 });
  });
});

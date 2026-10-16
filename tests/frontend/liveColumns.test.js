import { describe, expect, it } from "vitest";
import { shallowRef } from "vue";
import { visibleColumns } from "../../src/lib/columns.js";
import { tableSource } from "../../src/lib/gridSources.js";

/**
 * The bug: the grid read a snapshot of the columns while the explorer cleared
 * its own list, so the grid kept rendering indexes the editor no longer had
 * and every cell read threw. The live list is now the single owner.
 */
describe("a table source shares the explorer's live columns", () => {
  const columns = shallowRef([{ name: "id" }, { name: "name" }]);
  const layout = { order: [0, 1], hidden: [] };
  const source = tableSource({
    profileId: "p",
    tabId: "t",
    target: { kind: "table" },
    columns,
  });

  it("starts from the list it was given", () => {
    expect(source.columns).toEqual([{ name: "id" }, { name: "name" }]);
    expect(visibleColumns(source.columns, layout)).toEqual([0, 1]);
  });

  it("follows the list when the explorer clears it", () => {
    columns.value = [];
    expect(source.columns).toEqual([]);
    // No stale indexes survive to be read from an empty list.
    expect(visibleColumns(source.columns, layout)).toEqual([]);
  });

  it("never offers an index the list does not have", () => {
    columns.value = [{ name: "id" }];
    for (const position of visibleColumns(source.columns, layout)) {
      expect(source.columns[position]).toBeDefined();
    }
  });
});

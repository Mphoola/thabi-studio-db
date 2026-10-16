import { describe, expect, it } from "vitest";
import { computed, nextTick, shallowRef } from "vue";
import {
  columnLayoutRef,
  useColumns,
} from "../../src/composables/useColumns.js";
import { useRowEdits } from "../../src/composables/useRowEdits.js";

/** Hiding a column is the dialog's checkbox; this is what it calls. */
describe("the columns dialog can hide and show a column", () => {
  const columns = [
    { name: "id", typeName: "INT" },
    { name: "name", typeName: "TEXT" },
    { name: "total", typeName: "INT" },
  ];
  function setup() {
    const tab = {};
    const layout = columnLayoutRef(tab);
    const source = shallowRef({ columns });
    const state = useColumns({
      columns: computed(() => source.value.columns),
      layout,
    });
    return { tab, layout, state };
  }

  it("hides and shows by name", () => {
    const { state } = setup();
    expect(state.visible.value).toEqual([0, 1, 2]);
    state.toggle("name");
    expect(state.visible.value).toEqual([0, 2]);
    expect(state.isHidden("name")).toBe(true);
    state.toggle("name");
    expect(state.visible.value).toEqual([0, 1, 2]);
    expect(state.isHidden("name")).toBe(false);
  });

  it("keeps the change on the tab", async () => {
    const { tab, state } = setup();
    state.toggle("name");
    await nextTick();
    expect(tab.columnLayout.hidden).toEqual([1]);
  });

  it("reorders by name", () => {
    const { state } = setup();
    state.move("total", -1);
    expect(state.visible.value).toEqual([0, 2, 1]);
  });

  it("ignores a column that is not there", () => {
    const { state } = setup();
    state.toggle("nope");
    expect(state.visible.value).toEqual([0, 1, 2]);
  });
});

/**
 * The editor is asked about columns during render. When the result's shape
 * changes under a grid that still holds rows of the old shape, the column the
 * grid names is not there. Throwing there took down the whole panel, so every
 * operation on an unknown column is a no-op instead.
 */
describe("an unknown column is a no-op, not a crash", () => {
  const rows = [{ name: "id" }, { name: "total" }];
  const tab = {};
  const columns = shallowRef(rows);
  const editor = useRowEdits(tab, columns, shallowRef(["id"]));
  const cells = [
    { kind: "value", value: "7" },
    { kind: "value", value: "70" },
  ];

  it("edits a column that exists", () => {
    expect(editor.canEdit(cells, 1)).toBe(true);
    editor.setValue(cells, 1, "99");
    expect(editor.display(cells, 1).value).toBe("99");
    expect(editor.isChanged(cells, 1)).toBe(true);
  });

  it("survives the result changing shape under the grid", () => {
    columns.value = rows.slice(0, 1); // "total" is gone
    expect(() => editor.display(cells, 1)).not.toThrow();
    expect(editor.display(cells, 1)).toBe(cells[1]); // the raw cell
    expect(editor.canEdit(cells, 1)).toBe(false);
    expect(editor.isChanged(cells, 1)).toBe(false);
    expect(() => editor.setValue(cells, 1, "x")).not.toThrow();
    // The column that is still there keeps working.
    expect(editor.display(cells, 0)).toEqual({ kind: "value", value: "7" });
  });
});

import { describe, expect, it } from "vitest";
import {
  defaultLayout,
  fitWidth,
  moveColumn,
  resetLayout,
  toggleHidden,
  visibleColumns,
} from "../../src/lib/columns.js";

const columns = [
  { name: "id", typeName: "integer" },
  { name: "name", typeName: "text" },
  { name: "total", typeName: "numeric" },
  { name: "note", typeName: "text" },
];
const names = (visible) => visible.map((index) => columns[index].name);

describe("visibleColumns", () => {
  it("shows every column in its own order by default", () => {
    expect(names(visibleColumns(columns, defaultLayout()))).toEqual([
      "id",
      "name",
      "total",
      "note",
    ]);
    expect(names(visibleColumns(columns, null))).toEqual([
      "id",
      "name",
      "total",
      "note",
    ]);
  });

  it("applies a stored order and hides what was hidden", () => {
    expect(
      names(visibleColumns(columns, { order: [2, 0, 1, 3], hidden: [1] })),
    ).toEqual(["total", "id", "note"]);
  });

  it("heals a stale layout instead of losing columns", () => {
    // Column 9 no longer exists, and a new column 4 appeared.
    expect(
      names(visibleColumns(columns, { order: [9, 1, 0], hidden: [8] })),
    ).toEqual(["name", "id", "total", "note"]);
  });

  it("drops a stale hidden entry and keeps a real one", () => {
    expect(
      names(visibleColumns(columns, { order: [], hidden: [1, 99] })),
    ).toEqual(["id", "total", "note"]);
  });
});

describe("toggleHidden", () => {
  it("hides and shows a column by name", () => {
    const hidden = toggleHidden(columns, defaultLayout(), "total");
    expect(names(visibleColumns(columns, hidden))).toEqual([
      "id",
      "name",
      "note",
    ]);
    const shown = toggleHidden(columns, hidden, "total");
    expect(names(visibleColumns(columns, shown))).toEqual([
      "id",
      "name",
      "total",
      "note",
    ]);
  });

  it("ignores a column that is not there", () => {
    const layout = defaultLayout();
    expect(toggleHidden(columns, layout, "nope")).toBe(layout);
  });
});

describe("moveColumn", () => {
  it("swaps a column with its neighbour in the display order", () => {
    const moved = moveColumn(columns, defaultLayout(), "name", 1);
    expect(names(visibleColumns(columns, moved))).toEqual([
      "id",
      "total",
      "name",
      "note",
    ]);
    // And moving it back gives the original order.
    expect(
      names(visibleColumns(columns, moveColumn(columns, moved, "name", -1))),
    ).toEqual(["id", "name", "total", "note"]);
  });

  it("stops at the ends instead of wrapping", () => {
    const layout = defaultLayout();
    expect(moveColumn(columns, layout, "id", -1)).toBe(layout);
    expect(moveColumn(columns, layout, "note", 1)).toBe(layout);
  });

  it("keeps hidden columns out of the visible order", () => {
    const hidden = toggleHidden(columns, defaultLayout(), "name");
    expect(visibleColumns(columns, hidden)).not.toContain(1);
    // Visible order is id, total, note: moving note left swaps it with total.
    const moved = moveColumn(columns, hidden, "note", -1);
    expect(names(visibleColumns(columns, moved))).toEqual([
      "id",
      "note",
      "total",
    ]);
    // And the hidden column is still hidden after the move.
    expect(visibleColumns(columns, moved)).not.toContain(1);
  });
});

describe("fitWidth", () => {
  it("fits the widest loaded value and stays in range", () => {
    const short = [
      [1, "ab"],
      [2, "c"],
    ];
    const long = [[1, "a".repeat(60)]];
    expect(fitWidth(short, 1, "text")).toBeGreaterThan(60);
    expect(fitWidth(long, 1, "text")).toBe(400);
    expect(fitWidth([], 0, "text")).toBe(90);
  });
});

describe("resetLayout", () => {
  it("brings every column back", () => {
    expect(names(visibleColumns(columns, resetLayout()))).toEqual([
      "id",
      "name",
      "total",
      "note",
    ]);
  });
});

/**
 * The grid addresses display positions and maps them to source columns for
 * everything that reads a real cell: editing, the value inspector, the row
 * panel, and the copy scope. Hiding or reordering a column is what makes the
 * difference visible, so the mapping is tested where the other bug was not.
 */
describe("display positions map to source columns", () => {
  const hidden = toggleHidden(columns, defaultLayout(), "name");
  const visible = visibleColumns(columns, hidden);

  it("shows the remaining columns, in order", () => {
    expect(names(visible)).toEqual(["id", "total", "note"]);
  });

  it("reads a cell through the source column, not the position", () => {
    const cells = [["7", "hidden", "70", "note"]];
    // Display position 2 is the third shown column, "note" at source 3.
    expect(visible[2]).toBe(3);
    expect(cells[0][visible[2]]).toBe("note");
    // Reading by display position would have returned the wrong column.
    expect(cells[0][2]).toBe("70");
  });

  it("maps a copy selection to source columns", () => {
    const toSource = (position) => visible[position] ?? position;
    const selection = [0, 1, 2].map(toSource);
    expect(selection).toEqual([0, 2, 3]);
    // The middle selection is "total", not the hidden "name".
    expect(columns[selection[1]].name).toBe("total");
  });

  it("survives a reorder as well as a hide", () => {
    // "total" is at position 2, so -2 takes it to the front.
    const moved = moveColumn(columns, defaultLayout(), "total", -2);
    const order = visibleColumns(columns, moved);
    expect(order.map((index) => columns[index].name)).toEqual([
      "total",
      "id",
      "name",
      "note",
    ]);
    expect(columns[order[0]].name).toBe("total");
  });
});

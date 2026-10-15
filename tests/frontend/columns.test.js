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

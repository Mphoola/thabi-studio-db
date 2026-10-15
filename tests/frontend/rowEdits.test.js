import { describe, expect, it } from "vitest";
import {
  counts,
  createEdits,
  keyParts,
  revertRow,
  rowKey,
  setCell,
  shownCell,
  toChanges,
  toggleDeletes,
} from "../../src/lib/rowEdits.js";

const columns = [{ name: "id" }, { name: "note" }];
const v = (value) => ({ kind: "value", value });
const row = (id, note) => [v(id), note === null ? { kind: "null" } : v(note)];

describe("row edits", () => {
  it("identifies rows by primary key, and only plain-text keys", () => {
    expect(keyParts(row("7", "a"), columns, [0])).toEqual([
      { column: "id", value: "7" },
    ]);
    expect(keyParts([{ kind: "null" }], columns, [0])).toBeNull();
    expect(keyParts(row("7", "a"), columns, [])).toBeNull();
  });

  it("stages cell changes and drops them when set back", () => {
    const edits = createEdits();
    const parts = keyParts(row("7", "a"), columns, [0]);
    setCell(edits, parts, "note", v("a"), "b");
    expect(shownCell(edits, rowKey(parts), "note", v("a"))).toEqual(v("b"));
    setCell(edits, parts, "note", v("a"), null);
    expect(shownCell(edits, rowKey(parts), "note", v("a"))).toEqual({
      kind: "null",
    });
    setCell(edits, parts, "note", v("a"), "a");
    expect(counts(edits).total).toBe(0);
  });

  it("toggles deletes and builds changes in order", () => {
    const edits = createEdits();
    const one = keyParts(row("1", "a"), columns, [0]);
    const two = keyParts(row("2", "b"), columns, [0]);
    setCell(edits, one, "note", v("a"), "z");
    setCell(edits, two, "note", v("b"), "y");
    toggleDeletes(edits, [two]);
    edits.inserts.push({ id: "3" });
    expect(counts(edits)).toEqual({
      edited: 1,
      deleted: 1,
      inserted: 1,
      total: 3,
    });
    expect(toChanges(edits)).toEqual([
      { kind: "delete", key: two },
      { kind: "update", key: one, set: [{ column: "note", value: "z" }] },
      { kind: "insert", values: [{ column: "id", value: "3" }] },
    ]);
    toggleDeletes(edits, [two]);
    revertRow(edits, one);
    expect(counts(edits).deleted).toBe(0);
    expect(counts(edits).edited).toBe(1);
  });
});

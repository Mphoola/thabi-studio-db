import { describe, expect, it } from "vitest";
import {
  lineIsDimmed,
  lineIsLit,
  lineTouches,
  tableIsDimmed,
} from "../../src/lib/erHighlight.js";

/** Two tables joined by one key. */
const line = { id: "fk", fromTable: "orders", toTable: "customers" };

describe("a line knows its own tables", () => {
  it("touches the table it leaves and the table it arrives at", () => {
    expect(lineTouches(line, "orders")).toBe(true);
    expect(lineTouches(line, "customers")).toBe(true);
  });

  it("does not touch anything else", () => {
    expect(lineTouches(line, "invoices")).toBe(false);
  });
});

/**
 * `null is not an object (evaluating 'line.fromTable')`.
 *
 * This was one function taking a table *or* a line, so one caller passed null
 * where the other read a property. Written as four functions with one shape
 * each, and checked — because "it cannot be null any more" is exactly the kind
 * of thing that is true until somebody passes null.
 */
describe("nothing here throws on the wrong input", () => {
  for (const wrong of [null, undefined, {}, 0, "", "line", []]) {
    it(`survives ${JSON.stringify(wrong) ?? String(wrong)}`, () => {
      expect(() => lineTouches(wrong, "orders")).not.toThrow();
      expect(() => lineIsDimmed(wrong, "orders")).not.toThrow();
      expect(() => lineIsLit(wrong, "orders")).not.toThrow();
      expect(() => tableIsDimmed(wrong, "orders")).not.toThrow();
    });
  }
});

describe("while a table is pointed at", () => {
  it("its own keys stay lit", () => {
    expect(lineIsLit(line, "orders")).toBe(true);
    expect(lineIsDimmed(line, "orders")).toBe(false);
  });

  it("keys between other tables stand back", () => {
    const other = { id: "fk2", fromTable: "invoices", toTable: "suppliers" };
    expect(lineIsLit(other, "orders")).toBe(false);
    expect(lineIsDimmed(other, "orders")).toBe(true);
  });

  it("its own box stays lit, other boxes stand back", () => {
    expect(tableIsDimmed("orders", "orders")).toBe(false);
    expect(tableIsDimmed("invoices", "orders")).toBe(true);
  });
});

describe("while nothing is pointed at", () => {
  it("nothing is dimmed, because dimming everything reads as broken", () => {
    expect(lineIsDimmed(line, null)).toBe(false);
    expect(tableIsDimmed("orders", null)).toBe(false);
    expect(lineIsLit(line, null)).toBe(false);
  });
});

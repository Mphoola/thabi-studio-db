import { describe, expect, it } from "vitest";
import {
  DEFAULT_PAGE_SIZE,
  MAX_PAGE_SIZE,
  UNKNOWN_TOTAL,
  describeRows,
  pageCount,
  pageOf,
  tablePageSize,
} from "../../src/lib/tablePaging.js";

/**
 * A page is the chunk the user moves between, so its size decides how far one
 * request goes. It comes from the connection: 1,000 when the connection says
 * nothing, whatever it says when it says something, and never more than the
 * server will serve.
 */
describe("rows per page come from the connection", () => {
  it("shows a thousand rows when the connection sets no limit", () => {
    expect(tablePageSize(null)).toBe(1_000);
    expect(tablePageSize({})).toBe(1_000);
    expect(tablePageSize({ maxResultRows: 0 })).toBe(1_000);
    expect(tablePageSize({ maxResultRows: "" })).toBe(1_000);
    expect(tablePageSize({ maxResultRows: null })).toBe(1_000);
    expect(DEFAULT_PAGE_SIZE).toBe(1_000);
  });

  it("uses the connection's own amount", () => {
    expect(tablePageSize({ maxResultRows: 250 })).toBe(250);
    expect(tablePageSize({ maxResultRows: 10_000 })).toBe(10_000);
    expect(tablePageSize({ maxResultRows: "5000" })).toBe(5_000);
  });

  it("never asks for more than the server will serve", () => {
    expect(tablePageSize({ maxResultRows: 1_000_000 })).toBe(MAX_PAGE_SIZE);
    expect(MAX_PAGE_SIZE).toBe(100_000);
  });

  it("never asks for fewer than a row", () => {
    expect(tablePageSize({ maxResultRows: -5 })).toBe(1_000);
    expect(tablePageSize({ maxResultRows: 0.4 })).toBe(1);
  });
});

describe("where the user is in a table", () => {
  it("counts pages, rounding a short last page up", () => {
    expect(pageCount(1_100_000, 1_000)).toBe(1_100);
    expect(pageCount(1_000, 1_000)).toBe(1);
    expect(pageCount(1_001, 1_000)).toBe(2);
    expect(pageCount(0, 1_000)).toBe(1);
  });

  it("says nothing when the size is unknown", () => {
    // The stand-in a source uses when the estimate failed. It is finite, which
    // is exactly why reading it as a total would print nine quadrillion rows.
    expect(pageCount(UNKNOWN_TOTAL, 1_000)).toBeNull();
    expect(pageCount(Number.MAX_SAFE_INTEGER, 1_000)).toBeNull();
    expect(pageCount(null, 1_000)).toBeNull();
  });

  it("maps a row position to its page", () => {
    expect(pageOf(0, 1_000)).toBe(0);
    expect(pageOf(999, 1_000)).toBe(0);
    expect(pageOf(1_000, 1_000)).toBe(1);
    expect(pageOf(1_099_999, 1_000)).toBe(1_099);
  });
});

describe("what the pager says", () => {
  it("shows the rows on screen and the total", () => {
    expect(describeRows(0, 1_000, 1_100_000, true)).toBe(
      "Rows 1–1,000 of about 1,100,000",
    );
    // The page is zero-based, as the pager's "Page 3" is `page + 1`.
    expect(describeRows(2, 1_000, 1_100_000, true)).toBe(
      "Rows 2,001–3,000 of about 1,100,000",
    );
  });

  it("does not claim rows past the end", () => {
    expect(describeRows(1, 1_000, 1_500, false)).toBe(
      "Rows 1,001–1,500 of 1,500",
    );
    expect(describeRows(0, 1_000, 1_500, false)).toBe("Rows 1–1,000 of 1,500");
  });

  it("does not claim a total it does not know", () => {
    expect(describeRows(0, 1_000, UNKNOWN_TOTAL, true)).toBe("Rows 1–");
    expect(describeRows(0, 1_000, Infinity, false)).toBe("Rows 1–");
  });
});

import { describe, expect, it } from "vitest";
import { RAW_SQL, newFilter, toRequest } from "../../src/lib/rowFilters.js";

describe("toRequest", () => {
  it("keeps enabled, complete filters and maps raw SQL", () => {
    const filters = [
      { ...newFilter("id"), value: "71" },
      { ...newFilter("id"), value: "" }, // no value yet
      { ...newFilter("note"), op: "is_null" },
      { ...newFilter(RAW_SQL), value: "  a > 1 " },
      { ...newFilter(RAW_SQL), value: " " },
      { ...newFilter("x"), value: "1", enabled: false },
      newFilter(), // no column
    ];
    expect(toRequest(filters)).toEqual([
      { column: "id", op: "eq", value: "71" },
      { column: "note", op: "is_null", value: "" },
      { column: "", op: "raw", value: "a > 1" },
    ]);
  });
});

import { describe, expect, it } from "vitest";
import { autoMap, formatFor, toMappings } from "../../src/lib/importMapping.js";

describe("import mapping", () => {
  it("matches columns by name, ignoring case and punctuation", () => {
    expect(
      autoMap(
        ["Customer ID", "E-mail", "extra"],
        ["customer_id", "email", "created"],
      ),
    ).toEqual([0, 1, -1]);
  });

  it("leaves out unmapped columns", () => {
    expect(toMappings(["a", "b", "c"], [2, -1, 0])).toEqual([
      { column: "a", fileColumn: 2 },
      { column: "c", fileColumn: 0 },
    ]);
  });

  it("picks the format from the extension", () => {
    expect(formatFor("/x/rows.JSONL")).toBe("json");
    expect(formatFor("/x/rows.tsv")).toBe("csv");
  });
});

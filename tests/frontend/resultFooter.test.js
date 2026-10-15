import { describe, expect, it } from "vitest";
import { resultFooter } from "../../src/lib/resultFooter.js";

/**
 * A query result and a table page share a grid and disagree about what `total`
 * and `truncated` mean. Read with one set of rules they contradicted each
 * other: "First 1,109,100 rows fetched, add a WHERE or LIMIT" directly above
 * "Rows 1–1,000 of about 1,109,100".
 */
describe("the results bar says what actually happened", () => {
  const table = { kind: "table", total: 1_109_100, estimated: true };
  const result = { kind: "result", total: 5_000, estimated: false };

  it("leaves a table's row count to the pager", () => {
    // The page really was cut short at the limit, and that is not a problem to
    // warn about: the pager is how the rest of the table is reached.
    const footer = resultFooter(table, { truncated: true, elapsedMs: 42 });
    expect(footer.summary).toBe("");
    expect(footer.warnings).toEqual([]);
    expect(footer.elapsedMs).toBe("42 ms");
  });

  it("counts a query result's rows", () => {
    expect(resultFooter(result, { elapsedMs: 12 }).summary).toBe("5,000 rows");
    expect(resultFooter({ ...result, total: 1 }, {}).summary).toBe("1 row");
  });

  it("warns that a query result was cut short", () => {
    const footer = resultFooter(result, { truncated: true });
    expect(footer.summary).toBe("First 5,000 rows fetched");
    expect(footer.warnings).toEqual([
      "More rows exist. Add a WHERE or LIMIT, or raise the limit in Settings.",
    ]);
  });

  it("mentions extra result sets", () => {
    const footer = resultFooter(result, { extraResultSets: 2 });
    expect(footer.warnings).toEqual(["2 more result sets not shown"]);
  });

  it("says nothing about a time it was not given", () => {
    expect(resultFooter(result, {}).elapsedMs).toBe("");
    expect(resultFooter(table, {}).elapsedMs).toBe("");
    expect(resultFooter(result).elapsedMs).toBe("");
  });
});

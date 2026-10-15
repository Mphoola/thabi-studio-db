import { describe, expect, it } from "vitest";
import {
  findParameters,
  parameterLabels,
} from "../../src/lib/queryParameters.js";

/**
 * The property that matters is that a placeholder is replaced by the engine's
 * own marker and the value never enters the statement. So the rewritten SQL is
 * asserted directly: it must contain the value nowhere, and it must still be
 * the user's statement otherwise.
 */
describe("placeholders are found and rewritten for the engine", () => {
  it("rewrites named placeholders to the engine's own", () => {
    expect(
      findParameters("SELECT * FROM o WHERE s = :status", "postgres"),
    ).toEqual({
      count: 1,
      names: ["status"],
      sql: "SELECT * FROM o WHERE s = $1",
    });
    expect(
      findParameters("SELECT * FROM o WHERE s = :status", "mysql"),
    ).toEqual({
      count: 1,
      names: ["status"],
      sql: "SELECT * FROM o WHERE s = ?",
    });
    expect(
      findParameters("SELECT * FROM o WHERE s = :status", "sqlserver"),
    ).toEqual({
      count: 1,
      names: ["status"],
      sql: "SELECT * FROM o WHERE s = @p1",
    });
  });

  it("reads the engine's own placeholders as they stand", () => {
    expect(findParameters("SELECT * FROM o WHERE i = $1", "postgres")).toEqual({
      count: 1,
      names: [1],
      sql: "SELECT * FROM o WHERE i = $1",
    });
    expect(findParameters("SELECT * FROM o WHERE i = ?", "mysql")).toEqual({
      count: 1,
      names: [1],
      sql: "SELECT * FROM o WHERE i = ?",
    });
    expect(
      findParameters("SELECT * FROM o WHERE i = @status", "sqlserver"),
    ).toEqual({
      count: 1,
      names: ["status"],
      sql: "SELECT * FROM o WHERE i = @p1",
    });
  });

  it("numbers several placeholders in the order they bind", () => {
    const found = findParameters(
      "SELECT * FROM o WHERE s = :status AND t > :total",
      "postgres",
    );
    expect(found.names).toEqual(["status", "total"]);
    expect(found.sql).toBe("SELECT * FROM o WHERE s = $1 AND t > $2");
  });

  it("never lets a value into the statement", () => {
    const hostile = "'; DROP TABLE orders; --";
    const found = findParameters(
      "SELECT * FROM o WHERE s = :status",
      "postgres",
    );
    expect(found.sql).not.toContain(hostile);
    expect(found.sql).not.toContain("DROP");
    // The name is what travels, and it is not the value.
    expect(found.names).toEqual(["status"]);
  });

  it("ignores a placeholder inside a string", () => {
    expect(
      findParameters("SELECT ':not a param' AS note FROM o", "postgres"),
    ).toBeNull();
    expect(
      findParameters("SELECT * FROM o WHERE s = 'paid'", "postgres"),
    ).toBeNull();
  });

  it("ignores a placeholder inside a quoted identifier", () => {
    expect(findParameters('SELECT "weird:name" FROM o', "postgres")).toBeNull();
    expect(findParameters("SELECT `odd?name` FROM o", "mysql")).toBeNull();
    expect(findParameters("SELECT [odd@name] FROM o", "sqlserver")).toBeNull();
  });

  it("ignores a placeholder inside a comment", () => {
    expect(
      findParameters("SELECT 1 -- where s = :status\nFROM o", "postgres"),
    ).toBeNull();
    expect(
      findParameters("SELECT 1 /* :status ? @n */ FROM o", "postgres"),
    ).toBeNull();
  });

  it("is not fooled by a cast or a doubled quote", () => {
    expect(findParameters("SELECT x::int FROM o", "postgres")).toBeNull();
    expect(
      findParameters("SELECT * FROM o WHERE s = 'it''s :here'", "postgres"),
    ).toBeNull();
  });

  it("says there is nothing to ask when there is nothing", () => {
    expect(findParameters("SELECT 1", "postgres")).toBeNull();
    expect(findParameters("", "postgres")).toBeNull();
  });

  it("labels what the form should ask for", () => {
    expect(parameterLabels(["status", "total"])).toEqual(["status", "total"]);
    expect(parameterLabels([1, 2])).toEqual(["Parameter 1", "Parameter 2"]);
  });
});

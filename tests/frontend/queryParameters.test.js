import { describe, expect, it } from "vitest";
import {
  boundValues,
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
      order: [0],
      sql: "SELECT * FROM o WHERE s = $1",
    });
    expect(
      findParameters("SELECT * FROM o WHERE s = :status", "mysql").sql,
    ).toBe("SELECT * FROM o WHERE s = ?");
    expect(
      findParameters("SELECT * FROM o WHERE s = :status", "sqlserver").sql,
    ).toBe("SELECT * FROM o WHERE s = @P1");
  });

  it("reads the engine's own placeholders as they stand", () => {
    expect(findParameters("SELECT * FROM o WHERE i = $1", "postgres").sql).toBe(
      "SELECT * FROM o WHERE i = $1",
    );
    expect(findParameters("SELECT * FROM o WHERE i = ?", "mysql")).toEqual({
      count: 1,
      names: [1],
      order: [0],
      sql: "SELECT * FROM o WHERE i = ?",
    });
  });

  it("asks once for a name used twice", () => {
    const sql = "SELECT * FROM o WHERE a = :id OR b = :id";
    const pg = findParameters(sql, "postgres");
    expect(pg.names).toEqual(["id"]);
    expect(pg.sql).toBe("SELECT * FROM o WHERE a = $1 OR b = $1");
    const my = findParameters(sql, "mysql");
    expect(my.names).toEqual(["id"]);
    expect(my.order).toEqual([0, 0]);
    expect(boundValues(my, [{ type: "number", value: " 7 " }])).toEqual([
      { kind: "number", value: "7" },
      { kind: "number", value: "7" },
    ]);
  });

  it("leaves variables, operators and slices alone", () => {
    // T-SQL and MySQL variables, including @@ globals.
    expect(findParameters("SELECT @total, @@ROWCOUNT", "sqlserver")).toBeNull();
    expect(findParameters("SET @x = 1; SELECT @x", "mysql")).toBeNull();
    // PostgreSQL's JSON `?` operator and array slices.
    expect(findParameters("SELECT data ? 'key' FROM o", "postgres")).toBeNull();
    expect(findParameters("SELECT arr[1:n] FROM o", "postgres")).toBeNull();
    // MySQL assignment.
    expect(findParameters("SELECT @a := 1", "mysql")).toBeNull();
  });

  it("builds values with types, and NULL as NULL", () => {
    const found = findParameters("SELECT :a, :b, :c", "postgres");
    expect(
      boundValues(found, [
        { type: "text", value: "O'Brien" },
        { type: "number", value: "9007199254740993" },
        { type: "null", value: "" },
      ]),
    ).toEqual([
      { kind: "text", value: "O'Brien" },
      { kind: "number", value: "9007199254740993" },
      { kind: "null" },
    ]);
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

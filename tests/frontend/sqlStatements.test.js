import { describe, expect, it } from "vitest";
import { splitStatements, statementAt } from "../../src/lib/sqlStatements.js";

const texts = (sql, engine) => splitStatements(sql, engine).map((s) => s.text);

describe("splitStatements", () => {
  it("splits on semicolons and trims", () => {
    expect(texts("SELECT 1;\n  SELECT 2 ;")).toEqual(["SELECT 1", "SELECT 2"]);
  });

  it("ignores semicolons in strings, identifiers and comments", () => {
    const sql = `SELECT 'a;b', "x;y" FROM t; -- c;d\nSELECT 2 /* e;f */;`;
    expect(texts(sql)).toEqual([
      `SELECT 'a;b', "x;y" FROM t`,
      "-- c;d\nSELECT 2 /* e;f */",
    ]);
  });

  it("handles doubled quotes", () => {
    expect(texts("SELECT 'it''s; fine'; SELECT 2")).toEqual([
      "SELECT 'it''s; fine'",
      "SELECT 2",
    ]);
  });

  it("understands PostgreSQL dollar quoting", () => {
    const sql =
      "CREATE FUNCTION f() RETURNS int AS $body$ BEGIN; RETURN 1; END $body$ LANGUAGE plpgsql; SELECT f()";
    expect(texts(sql, "postgres")).toHaveLength(2);
  });

  it("understands MySQL backticks, # comments and backslash escapes", () => {
    const sql = "SELECT `a;b` FROM t; # x;y\nSELECT 'it\\'s; ok'";
    expect(texts(sql, "mysql")).toEqual([
      "SELECT `a;b` FROM t",
      "# x;y\nSELECT 'it\\'s; ok'",
    ]);
  });

  it("understands SQL Server brackets and GO lines", () => {
    const sql = "SELECT [a;b] FROM t\nGO\nSELECT 2\n  go  \nSELECT 3";
    expect(texts(sql, "sqlserver")).toEqual([
      "SELECT [a;b] FROM t",
      "SELECT 2",
      "SELECT 3",
    ]);
  });

  it("drops empty and comment-only pieces", () => {
    expect(texts(";;  -- only a comment\n;SELECT 1")).toEqual(["SELECT 1"]);
  });
});

describe("statementAt", () => {
  const sql = "SELECT 1;\n\nSELECT 2;";
  const statements = splitStatements(sql);

  it("finds the statement under the cursor", () => {
    expect(statementAt(statements, sql.indexOf("2")).text).toBe("SELECT 2");
    expect(statementAt(statements, 0).text).toBe("SELECT 1");
  });

  it("uses the previous statement between statements", () => {
    expect(statementAt(statements, sql.indexOf("\n\n") + 1).text).toBe(
      "SELECT 1",
    );
  });
});

describe("useTarget", () => {
  it("reads the database from USE statements", async () => {
    const { useTarget } = await import("../../src/lib/sqlStatements.js");
    expect(useTarget("USE shop")).toBe("shop");
    expect(useTarget("use `my db`;")).toBe("my db");
    expect(useTarget("USE [Sales Ü]")).toBe("Sales Ü");
    expect(useTarget("SELECT 1")).toBeNull();
  });
});

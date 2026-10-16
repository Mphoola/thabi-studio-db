import { describe, expect, it } from "vitest";
import { formatSql, minifySql } from "../../src/lib/sqlFormat.js";

describe("formatSql", () => {
  it("beautifies per dialect and keeps placeholders", async () => {
    const out = await formatSql(
      "select a,b from t where c = :status",
      "postgres",
    );
    expect(out).toBe("SELECT\n  a,\n  b\nFROM\n  t\nWHERE\n  c = :status");
    expect(await formatSql("select [a] from t", "sqlserver")).toContain("[a]");
    expect(await formatSql("select 1", "mysql", { keywordCase: "lower" })).toBe(
      "select\n  1",
    );
  });
});

describe("minifySql", () => {
  it("collapses whitespace but not inside strings or names", () => {
    expect(minifySql("SELECT  a,\n   'x   y'\nFROM \"my  table\"")).toBe(
      "SELECT a, 'x   y' FROM \"my  table\"",
    );
  });

  it("turns line comments into block comments so code survives", () => {
    expect(minifySql("SELECT 1 -- one\nFROM t")).toBe(
      "SELECT 1 /* one */ FROM t",
    );
  });

  it("keeps escaped quotes inside strings", () => {
    expect(minifySql("SELECT 'it''s   ok'")).toBe("SELECT 'it''s   ok'");
  });
});

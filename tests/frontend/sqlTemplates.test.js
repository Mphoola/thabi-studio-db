import { describe, expect, it } from "vitest";
import { findParameters } from "../../src/lib/queryParameters.js";
import { parameterName, sqlTemplate } from "../../src/lib/sqlTemplates.js";

const node = { database: "shop", schema: "public", name: "order items" };
const columns = [
  { name: "order id", primaryKey: 1 },
  { name: "line", primaryKey: 2 },
  { name: "qty", primaryKey: null },
];

describe("sql templates", () => {
  it("quotes names per engine and limits a SELECT", () => {
    expect(sqlTemplate("select", "postgres", node, columns)).toBe(
      'SELECT "order id",\n       "line",\n       "qty"\nFROM "public"."order items"\nLIMIT 100;',
    );
    expect(
      sqlTemplate("select", "sqlserver", { ...node, schema: "dbo" }, columns),
    ).toMatch(/^SELECT TOP 100 \[order id\]/);
  });

  it("keys UPDATE and DELETE by the whole primary key, as parameters", () => {
    const update = sqlTemplate("update", "mysql", node, columns);
    expect(update).toContain("SET `qty` = :qty");
    expect(update).toContain(
      "WHERE `order id` = :order_id\n  AND `line` = :line",
    );
    // Every value is a parameter the editor recognises and binds.
    expect(findParameters(update, "mysql").names).toEqual([
      "qty",
      "order_id",
      "line",
    ]);
  });

  it("refuses to guess a WHERE without a key", () => {
    const keyless = columns.map((c) => ({ ...c, primaryKey: null }));
    expect(sqlTemplate("delete", "postgres", node, keyless)).toMatch(
      /no primary key[\s\S]*WHERE \/\* condition \*\//,
    );
  });

  it("makes parameter names from any column name", () => {
    expect(parameterName("order id")).toBe("order_id");
    expect(parameterName("1st")).toBe("c_1st");
  });
});

import { describe, expect, it } from "vitest";
import {
  databaseNode,
  objectGroups,
  qualifiedName,
  quoteIdent,
  schemaNode,
  visibleRows,
} from "../../src/lib/explorerTree.js";

describe("objectGroups", () => {
  const schema = schemaNode("shop", "public");
  const groups = objectGroups(schema, [
    { name: "orders", kind: "table" },
    { name: "Big Orders", kind: "view" },
    { name: "customers", kind: "table" },
  ]);

  it("groups by kind in a fixed order and skips empty groups", () => {
    expect(groups.map((g) => [g.label, g.count])).toEqual([
      ["Tables", 2],
      ["Views", 1],
    ]);
  });

  it("gives every node a unique key", () => {
    const keys = groups.flatMap((g) => [
      g.key,
      ...g.children.map((c) => c.key),
    ]);
    expect(new Set(keys).size).toBe(keys.length);
  });
});

describe("visibleRows", () => {
  it("shows children only under expanded nodes, with depth", () => {
    const db = databaseNode("shop");
    const schema = schemaNode("shop", "public");
    const rows = visibleRows([db], new Set([db.key]), (key) =>
      key === db.key ? [schema] : undefined,
    );
    expect(rows.map((r) => [r.node.label, r.level])).toEqual([
      ["shop", 1],
      ["public", 2],
    ]);
    expect(visibleRows([db], new Set(), () => [schema])).toHaveLength(1);
  });
});

describe("quoting", () => {
  it("matches each engine's rules", () => {
    expect(quoteIdent("postgres", 'a"b')).toBe('"a""b"');
    expect(quoteIdent("mysql", "a`b")).toBe("`a``b`");
    expect(quoteIdent("sqlserver", "a]b")).toBe("[a]]b]");
  });

  it("builds qualified names per engine", () => {
    const node = { database: "shop", schema: "Sales Ü", name: "Order Details" };
    expect(qualifiedName("postgres", node)).toBe('"Sales Ü"."Order Details"');
    expect(qualifiedName("sqlserver", node)).toBe(
      "[shop].[Sales Ü].[Order Details]",
    );
    expect(qualifiedName("mysql", { ...node, schema: null })).toBe(
      "`shop`.`Order Details`",
    );
  });
});

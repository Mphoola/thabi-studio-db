import { describe, expect, it } from "vitest";
import {
  databaseNode,
  diagramTargetFor,
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

/**
 * Where a diagram should be drawn.
 *
 * The nodes are named by `label` and carry no `name` at all, which is easy to
 * assume and draws a diagram of "undefined" rather than failing.
 */
describe("a diagram's target", () => {
  it("uses a database node's own database", () => {
    expect(diagramTargetFor(databaseNode("hr"))).toEqual({
      database: "hr",
      schema: null,
      title: "hr diagram",
    });
  });

  it("uses a schema node's schema, not its database", () => {
    expect(diagramTargetFor(schemaNode("shop", "public"))).toEqual({
      database: "shop",
      schema: "public",
      title: "public diagram",
    });
  });

  it("names a schema node by its label, which is where the name lives", () => {
    const node = schemaNode("shop", "sales");
    expect(node.name).toBeUndefined();
    expect(diagramTargetFor(node).schema).toBe("sales");
    expect(diagramTargetFor(node).database).toBe("shop");
  });

  it("says no rather than half-filling, so the caller can explain", () => {
    // A tab opened with no database is a blank panel and no explanation.
    expect(diagramTargetFor(databaseNode(undefined))).toBeNull();
    expect(diagramTargetFor({ type: "database" })).toBeNull();
  });

  it("draws nothing for a node that is not a database or a schema", () => {
    expect(diagramTargetFor(null)).toBeNull();
    expect(diagramTargetFor({ type: "object", name: "orders" })).toBeNull();
    expect(diagramTargetFor({ type: "group" })).toBeNull();
  });
});

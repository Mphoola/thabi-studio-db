import { describe, expect, it } from "vitest";
import {
  maskStrings,
  qualifyPrefix,
  resolveQualifier,
  statementTables,
} from "../../src/lib/sqlAliases.js";

/**
 * Typing `o.` in `FROM orders o` should offer orders' columns. That needs the
 * names the statement itself introduces, resolved through the alias the user
 * typed — not every table in the schema.
 */
describe("the names a statement gives its tables", () => {
  it("reads a table and its alias", () => {
    expect(statementTables("SELECT * FROM orders o")).toEqual([
      { table: "orders", schema: "", alias: "o" },
    ]);
    expect(statementTables("SELECT * FROM orders AS o")).toEqual([
      { table: "orders", schema: "", alias: "o" },
    ]);
    expect(statementTables("SELECT * FROM orders")).toEqual([
      { table: "orders", schema: "", alias: "" },
    ]);
  });

  it("reads every table in a join", () => {
    expect(
      statementTables(
        "SELECT * FROM orders o JOIN customers c ON o.cid = c.id",
      ),
    ).toEqual([
      { table: "orders", schema: "", alias: "o" },
      { table: "customers", schema: "", alias: "c" },
    ]);
    expect(
      statementTables("SELECT * FROM a LEFT JOIN b ON a.id = b.id").map(
        (t) => t.table,
      ),
    ).toEqual(["a", "b"]);
  });

  it("reads a comma list", () => {
    expect(
      statementTables("SELECT * FROM orders o, customers c").map(
        (t) => t.alias,
      ),
    ).toEqual(["o", "c"]);
  });

  it("reads a qualified name", () => {
    expect(statementTables("SELECT * FROM public.orders o")).toEqual([
      { table: "orders", schema: "public", alias: "o" },
    ]);
  });

  it("reads a quoted name", () => {
    expect(statementTables('SELECT * FROM "Order Items" oi')).toEqual([
      { table: "Order Items", schema: "", alias: "oi" },
    ]);
    expect(statementTables("SELECT * FROM `shop`.`orders`")).toEqual([
      { table: "orders", schema: "shop", alias: "" },
    ]);
  });

  it("is not fooled by a FROM inside a string or a comment", () => {
    expect(
      statementTables("SELECT 'from orders o' AS note").map((t) => t.table),
    ).toEqual([]);
    expect(
      statementTables("SELECT 1 -- from orders o\n").map((t) => t.table),
    ).toEqual([]);
  });

  it("does not read a join condition as a table", () => {
    expect(
      statementTables("SELECT * FROM orders o JOIN customers c ON o.id = c.id")
        .length,
    ).toBe(2);
    expect(
      statementTables("SELECT * FROM orders WHERE o.total > 1").length,
    ).toBe(1);
  });

  it("masks a string without losing the rest", () => {
    expect(maskStrings("SELECT 'it''s' FROM orders")).toContain("FROM orders");
    expect(maskStrings("SELECT 'from x' FROM orders")).not.toContain("from x");
  });
});

describe("what a qualifier before the dot refers to", () => {
  const tables = statementTables(
    "SELECT * FROM orders o JOIN customers c ON o.cid = c.id",
  );

  it("finds a table by its alias", () => {
    expect(resolveQualifier(tables, "o")).toMatchObject({ table: "orders" });
    expect(resolveQualifier(tables, "c")).toMatchObject({ table: "customers" });
  });

  it("finds a table by its own name, even when aliased", () => {
    expect(resolveQualifier(tables, "orders")).toMatchObject({
      table: "orders",
    });
    expect(resolveQualifier(tables, "customers")).toMatchObject({
      table: "customers",
    });
  });

  it("is not case about it", () => {
    expect(resolveQualifier(tables, "O")).toMatchObject({ table: "orders" });
  });

  it("has nothing for a name that is not there", () => {
    expect(resolveQualifier(tables, "x")).toBeNull();
    expect(resolveQualifier(tables, "")).toBeNull();
    expect(resolveQualifier([], "o")).toBeNull();
  });

  it("splits a typed prefix into the qualifier and what follows", () => {
    expect(qualifyPrefix(tables, "o.na")).toEqual({
      qualifier: "o",
      rest: "na",
    });
    expect(qualifyPrefix(tables, "o.")).toEqual({ qualifier: "o", rest: "" });
    // No dot yet: everything is what is being typed.
    expect(qualifyPrefix(tables, "na")).toEqual({ qualifier: "", rest: "na" });
  });
});

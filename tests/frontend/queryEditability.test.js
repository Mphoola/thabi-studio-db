import { describe, expect, it } from "vitest";
import {
  resultEditability,
  singleTableSelect,
  repeatedKeys,
} from "../../src/lib/queryEditability.js";

/**
 * Editing a result writes UPDATE … WHERE key = …. A wrong "yes" here writes to
 * a row the user is not looking at, so every refusal below is deliberate and the
 * strict cases are tested as refusals, not as things to be clever about later.
 */
describe("only a plain read of one table can be edited", () => {
  it("reads the table out of a simple select", () => {
    expect(singleTableSelect("SELECT id, name FROM orders")).toMatchObject({
      table: "orders",
    });
    expect(
      singleTableSelect("select id from public.orders where status = 'open'"),
    ).toMatchObject({ table: "orders", schema: "public" });
    expect(singleTableSelect("SELECT id FROM `shop`.`orders`")).toMatchObject({
      table: "orders",
      schema: "shop",
    });
    expect(singleTableSelect('SELECT id FROM "Orders"')).toMatchObject({
      table: "Orders",
    });
    expect(singleTableSelect("SELECT id FROM [dbo].[orders]")).toMatchObject({
      table: "orders",
      schema: "dbo",
    });
  });

  it("keeps up with clauses after the table", () => {
    for (const tail of [
      " WHERE status = 'open'",
      " ORDER BY id DESC",
      " LIMIT 10 OFFSET 5",
      " FETCH NEXT 10 ROWS ONLY",
      " WHERE id = 1 AND total > 0 ORDER BY id",
    ])
      expect(singleTableSelect(`SELECT id FROM orders${tail}`)).toMatchObject({
        table: "orders",
      });
  });

  it("does not read a string that looks like a clause", () => {
    expect(
      singleTableSelect("SELECT id FROM orders WHERE note = 'from nowhere'"),
    ).toMatchObject({ table: "orders" });
    expect(
      singleTableSelect("SELECT id FROM notes WHERE body = 'select x from y'"),
    ).toMatchObject({ table: "notes" });
  });

  it("ignores comments", () => {
    expect(
      singleTableSelect("SELECT id -- from orders\nFROM orders"),
    ).toMatchObject({ table: "orders" });
    expect(
      singleTableSelect("/* join nothing */ SELECT id FROM orders"),
    ).toMatchObject({ table: "orders" });
  });

  it("refuses anything that is not one table read plainly", () => {
    const refused = [
      "SELECT o.id FROM orders o JOIN customers c ON c.id = o.customer_id",
      "SELECT id FROM orders, customers",
      "SELECT id FROM (SELECT id FROM orders) x",
      "SELECT id FROM orders UNION SELECT id FROM archived_orders",
      "SELECT COUNT(*) FROM orders",
      "SELECT DISTINCT customer_id FROM orders",
      "SELECT customer_id, COUNT(*) FROM orders GROUP BY customer_id",
      "WITH recent AS (SELECT id FROM orders) SELECT id FROM recent",
      "SELECT id FROM orders WHERE id IN (SELECT id FROM archived)",
      "SELECT id, ROW_NUMBER() OVER (ORDER BY id) FROM orders",
      "SELECT id FROM orders HAVING COUNT(*) > 1",
      "UPDATE orders SET status = 'x'",
      "SELECT 1",
      "",
    ];
    for (const statement of refused)
      expect(singleTableSelect(statement), statement).toBeNull();
  });

  it("refuses a table name it cannot make sense of", () => {
    expect(singleTableSelect("SELECT id FROM orders; DROP TABLE x")).toBeNull();
    expect(singleTableSelect("SELECT id FROM (")).toBeNull();
    expect(singleTableSelect("SELECT id FROM orders AS")).toBeNull();
  });
});

describe("a result is only editable when its rows can be addressed", () => {
  const columns = [{ name: "id" }, { name: "name" }, { name: "total" }];
  const rows = [
    ["1", "a", "10"],
    ["2", "b", "20"],
  ];
  const base = {
    statement: "SELECT id, name, total FROM orders",
    columns,
    primaryKey: ["id"],
    rows,
  };

  it("accepts one row per table row with the whole key", () => {
    const answer = resultEditability(base);
    expect(answer.editable).toBe(true);
    expect(answer.table).toMatchObject({ table: "orders" });
  });

  it("says why a join cannot be edited", () => {
    const answer = resultEditability({
      ...base,
      statement:
        "SELECT o.id FROM orders o JOIN customers c ON c.id = o.customer_id",
    });
    expect(answer.editable).toBe(false);
    expect(answer.reason).toMatch(/one table/i);
  });

  it("says which key column is missing", () => {
    const answer = resultEditability({
      ...base,
      columns: [{ name: "name" }, { name: "total" }],
      primaryKey: ["tenant_id", "id"],
    });
    expect(answer.editable).toBe(false);
    expect(answer.reason).toContain("tenant_id");
    expect(answer.reason).toContain("id");
  });

  it("says why a repeated key cannot be edited", () => {
    const answer = resultEditability({
      ...base,
      rows: [
        ["1", "a", "10"],
        ["1", "b", "20"],
      ],
    });
    expect(answer.editable).toBe(false);
    expect(answer.reason).toMatch(/more than once/i);
  });

  it("notices a repeated key across a composite key", () => {
    const composite = [{ name: "tenant_id" }, { name: "id" }, { name: "name" }];
    expect(
      repeatedKeys(
        composite,
        ["tenant_id", "id"],
        [
          ["t1", "1", "a"],
          ["t1", "1", "b"],
        ],
      ),
    ).toBe(true);
    expect(
      repeatedKeys(
        composite,
        ["tenant_id", "id"],
        [
          ["t1", "1", "a"],
          ["t2", "1", "b"],
        ],
      ),
    ).toBe(false);
  });

  it("refuses when the key is not in the result at all", () => {
    const answer = resultEditability({
      ...base,
      columns: [{ name: "name" }],
      primaryKey: ["id"],
    });
    expect(answer.editable).toBe(false);
    expect(answer.reason).toContain("id");
  });
});

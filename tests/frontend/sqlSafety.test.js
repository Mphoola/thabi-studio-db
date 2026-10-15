import { describe, expect, it } from "vitest";
import { classifyStatement, confirmationFor } from "../../src/lib/sqlSafety.js";

const ctx = { environment: "local", database: "shop", connection: "Local" };

describe("classifyStatement", () => {
  it("treats reads as safe", () => {
    expect(
      classifyStatement("SELECT * FROM t WHERE x = 'DELETE'").changesData,
    ).toBe(false);
    expect(classifyStatement("-- note\nshow tables").changesData).toBe(false);
  });

  it("spots writes, including inside a CTE", () => {
    expect(classifyStatement("insert into t values (1)").changesData).toBe(
      true,
    );
    expect(
      classifyStatement("WITH x AS (DELETE FROM t RETURNING *) SELECT * FROM x")
        .changesData,
    ).toBe(true);
  });

  it("flags UPDATE and DELETE without WHERE", () => {
    expect(
      classifyStatement("UPDATE orders SET paid = true").risk,
    ).toMatchObject({
      kind: "no-where",
      verb: "UPDATE",
      target: "orders",
    });
    expect(classifyStatement('delete from "Order Details"').risk).toMatchObject(
      {
        target: "Order Details",
      },
    );
    expect(classifyStatement("DELETE FROM t WHERE id = 1").risk).toBeNull();
  });

  it("flags DROP and TRUNCATE", () => {
    expect(classifyStatement("DROP TABLE IF EXISTS `old`").risk).toMatchObject({
      kind: "drop",
      target: "old",
    });
    expect(classifyStatement("truncate table [dbo].[logs]").risk).toMatchObject(
      { kind: "truncate", target: "dbo.logs" },
    );
  });
});

describe("confirmationFor", () => {
  it("asks before risky statements anywhere", () => {
    expect(confirmationFor(["UPDATE orders SET x = 1"], ctx)).toMatchObject({
      title: "Update every row in “orders”?",
      confirmLabel: "Update All Rows",
    });
  });

  it("asks before any write on Production only", () => {
    const write = ["INSERT INTO t VALUES (1)"];
    expect(confirmationFor(write, ctx)).toBeNull();
    expect(
      confirmationFor(write, { ...ctx, environment: "production" }),
    ).toMatchObject({
      title: "Run on Production?",
    });
    expect(
      confirmationFor(["SELECT 1"], { ...ctx, environment: "production" }),
    ).toBeNull();
  });
});

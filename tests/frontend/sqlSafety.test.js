import { describe, expect, it } from "vitest";
import {
  applyConfirmationFor,
  classifyStatement,
  confirmationFor,
  defaultSafeMode,
  writeBlockFor,
} from "../../src/lib/sqlSafety.js";

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

  it("asks before any write when the mode confirms writes", () => {
    const write = ["INSERT INTO t VALUES (1)"];
    expect(confirmationFor(write, ctx)).toBeNull();
    expect(
      confirmationFor(write, { ...ctx, safeMode: "confirm-writes" }),
    ).toMatchObject({ title: "Run this statement?" });
    expect(
      confirmationFor(["SELECT 1"], { ...ctx, safeMode: "confirm-writes" }),
    ).toBeNull();
  });

  it("keeps Production asking and names the database", () => {
    expect(defaultSafeMode("production")).toBe("confirm-writes");
    expect(defaultSafeMode("local")).toBe("off");
    expect(
      confirmationFor(["INSERT INTO t VALUES (1)"], {
        ...ctx,
        environment: "production",
        safeMode: "confirm-writes",
      }),
    ).toMatchObject({
      title: "Run on Production?",
      message: expect.stringContaining("“shop”"),
    });
    expect(
      confirmationFor(["SELECT 1"], {
        ...ctx,
        environment: "production",
        safeMode: "confirm-writes",
      }),
    ).toBeNull();
  });

  it("asks about every statement in confirm-everything", () => {
    for (const sql of ["SELECT 1", "BEGIN"]) {
      expect(
        confirmationFor([sql], { ...ctx, safeMode: "confirm-everything" }),
      ).toMatchObject({
        title: "Run this statement?",
      });
    }
  });

  it("asks for destructive statements in every mode, typed on Production", () => {
    const drop = ["DROP TABLE orders"];
    for (const safeMode of ["off", "confirm-writes", "confirm-everything"]) {
      expect(confirmationFor(drop, { ...ctx, safeMode })).toMatchObject({
        title: "Drop “orders”?",
        requireText: null,
      });
    }
    expect(
      confirmationFor(drop, {
        ...ctx,
        environment: "production",
        safeMode: "off",
      }),
    ).toMatchObject({ title: "Drop “orders”?", requireText: "shop" });
  });

  it("still asks about a destructive statement on a Production connection set to off", () => {
    // Safety of the last resort: the risk check doesn't depend on safe mode.
    expect(
      confirmationFor(["DELETE FROM orders"], {
        ...ctx,
        environment: "production",
        safeMode: "off",
      }),
    ).toMatchObject({
      title: "Delete every row in “orders”?",
      requireText: "shop",
    });
  });
});

describe("writeBlockFor", () => {
  it("refuses writes on a read-only connection only", () => {
    expect(writeBlockFor(["INSERT INTO t VALUES (1)"], "read-only")).toContain(
      "read-only",
    );
    expect(writeBlockFor(["DROP TABLE t"], "read-only")).toContain("read-only");
    expect(writeBlockFor(["SELECT 1"], "read-only")).toBeNull();
    for (const safeMode of ["off", "confirm-writes", "confirm-everything"]) {
      expect(writeBlockFor(["DELETE FROM t"], safeMode)).toBeNull();
    }
  });
});

describe("applyConfirmationFor", () => {
  const context = {
    connection: "Prod",
    table: "orders",
    environment: "staging",
  };

  it("asks before any change in confirm-everything", () => {
    expect(
      applyConfirmationFor(
        { total: 2, deleted: 0 },
        {
          ...context,
          safeMode: "confirm-everything",
        },
      ),
    ).toMatchObject({ confirmLabel: "Apply Changes" });
  });

  it("asks before deletes whatever the mode is", () => {
    expect(
      applyConfirmationFor(
        { total: 1, deleted: 1 },
        { ...context, safeMode: "off" },
      ),
    ).toMatchObject({
      title: "Delete 1 row from “orders”?",
      destructive: true,
    });
    expect(
      applyConfirmationFor(
        { total: 1, deleted: 1 },
        {
          ...context,
          environment: "production",
          safeMode: "confirm-writes",
        },
      ),
    ).toMatchObject({ requireText: "orders" });
  });

  it("applies ordinary edits straight away when the mode is off", () => {
    expect(
      applyConfirmationFor(
        { total: 1, deleted: 0 },
        { ...context, safeMode: "off" },
      ),
    ).toBeNull();
  });
});

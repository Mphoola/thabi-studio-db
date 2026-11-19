import { describe, expect, it } from "vitest";
import {
  createPlan,
  createProblem,
  startingRows,
  typesFor,
} from "../../src/lib/createTable.js";

describe("create table", () => {
  it("starts with a key column in the engine's own types", () => {
    const [id, next] = startingRows("sqlserver");
    expect(id).toMatchObject({ name: "id", type: "INT", key: true });
    expect(next.type).toBe("NVARCHAR(MAX)");
    expect(typesFor("unknown")).toEqual(typesFor("postgres"));
  });

  it("says what is missing, in the order to fix it", () => {
    expect(createProblem("", startingRows("mysql"))).toMatch(/name/);
    expect(createProblem("t", [])).toMatch(/at least one column/);
    expect(
      createProblem("t", [
        { name: "a", type: "INT" },
        { name: "A", type: "INT" },
      ]),
    ).toMatch(/two columns called A/);
    expect(createProblem("t", [{ name: "a", type: " " }])).toMatch(/type/);
  });

  it("allows a key on several columns, none, and defaults", () => {
    const plan = createPlan("orders", [
      { name: "tenant", type: "INT", nullable: true, key: true, default: "" },
      { name: "id", type: "INT", nullable: false, key: true, default: "" },
      {
        name: "status",
        type: "TEXT",
        nullable: false,
        key: false,
        default: "'new'",
      },
      { name: " ", type: "TEXT", nullable: true, key: false, default: "" },
    ]);
    expect(plan.create.columns).toEqual([
      {
        name: "tenant",
        dataType: "INT",
        nullable: false,
        default: "",
        primaryKey: true,
      },
      {
        name: "id",
        dataType: "INT",
        nullable: false,
        default: "",
        primaryKey: true,
      },
      {
        name: "status",
        dataType: "TEXT",
        nullable: false,
        default: "'new'",
        primaryKey: false,
      },
    ]);
    expect(
      createPlan("heap", [
        { name: "x", type: "INT", nullable: true, key: false, default: "" },
      ]),
    ).not.toBeNull();
  });
});

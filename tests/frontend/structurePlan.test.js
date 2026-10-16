import { describe, expect, it } from "vitest";
import {
  describeEdit,
  sameEdit,
  stageChange,
  toPlan,
} from "../../src/lib/structurePlan.js";

const structure = {
  columns: [
    { name: "id", dataType: "INTEGER", nullable: false, primaryKey: 1 },
    { name: "note", dataType: "TEXT", nullable: true },
    {
      name: "status",
      dataType: "VARCHAR(20)",
      nullable: false,
      default: "'new'",
    },
  ],
  indexes: [
    { name: "orders_pkey", columns: ["id"], unique: true, primary: true },
    {
      name: "orders_status_idx",
      columns: ["status"],
      unique: false,
      primary: false,
    },
  ],
};

/**
 * A rename restates the whole column on two of the three engines, so the type
 * has to travel with it. A rename that silently blanked the type would be a
 * rename that changed something else.
 */
describe("a staged change becomes one plan entry", () => {
  it("carries the type through a rename", () => {
    const plan = toPlan(structure, [
      { kind: "renameColumn", from: "note", to: "summary" },
    ]);
    expect(plan.columns).toEqual([
      {
        renameColumn: {
          from: "note",
          to: "summary",
          spec: {
            name: "summary",
            dataType: "TEXT",
            nullable: true,
            default: "",
            comment: "",
            primaryKey: false,
            unique: false,
          },
        },
      },
    ]);
  });

  it("carries the type into a nullability change", () => {
    // MySQL and SQL Server restate the column to change nullability, so a
    // missing type would generate SQL that names nothing.
    const plan = toPlan(structure, [
      {
        kind: "setNullable",
        name: "status",
        nullable: true,
        dataType: "VARCHAR(20)",
      },
    ]);
    expect(plan.columns[0].setNullable).toEqual({
      name: "status",
      nullable: true,
      dataType: "VARCHAR(20)",
    });
  });

  it("keeps several changes in the order they were made", () => {
    const plan = toPlan(structure, [
      {
        kind: "addColumn",
        column: { name: "extra", dataType: "TEXT", nullable: true },
      },
      { kind: "dropColumn", name: "note" },
      { kind: "setType", name: "status", dataType: "TEXT" },
    ]);
    expect(plan.columns.map((one) => Object.keys(one)[0])).toEqual([
      "addColumn",
      "dropColumn",
      "setType",
    ]);
  });

  it("drops an index whose column is going", () => {
    // Every engine refuses to drop a column an index still points at, so a
    // plan that left it would show SQL that cannot run.
    const plan = toPlan(structure, [{ kind: "dropColumn", name: "status" }]);
    expect(plan.indexes).toContainEqual({
      drop: { name: "orders_status_idx" },
    });
  });

  it("leaves the primary key alone when another column goes", () => {
    const plan = toPlan(structure, [{ kind: "dropColumn", name: "note" }]);
    expect(plan.indexes).not.toContainEqual({ drop: { name: "orders_pkey" } });
  });

  it("drops nothing it does not understand rather than sending it", () => {
    // Showing SQL for a change that will not run is worse than not offering it.
    const plan = toPlan(structure, [{ kind: "somethingNew", name: "note" }]);
    expect(plan.columns).toEqual([]);
  });

  it("has no keys until keys can be staged", () => {
    expect(toPlan(structure, []).keys).toEqual([]);
  });
});

describe("staging the same change twice", () => {
  it("replaces it rather than sending it twice", () => {
    let edits = [];
    edits = stageChange(edits, {
      kind: "setType",
      name: "note",
      dataType: "TEXT",
    });
    edits = stageChange(edits, {
      kind: "setType",
      name: "note",
      dataType: "VARCHAR(50)",
    });
    expect(edits).toHaveLength(1);
    expect(edits[0].dataType).toBe("VARCHAR(50)");
  });

  it("keeps changes to different columns apart", () => {
    let edits = [];
    edits = stageChange(edits, {
      kind: "setType",
      name: "note",
      dataType: "TEXT",
    });
    edits = stageChange(edits, {
      kind: "setType",
      name: "status",
      dataType: "TEXT",
    });
    expect(edits).toHaveLength(2);
  });

  it("does not confuse a rename with a type change on the same column", () => {
    expect(
      sameEdit(
        { kind: "renameColumn", from: "note", to: "summary" },
        { kind: "setType", name: "note" },
      ),
    ).toBe(false);
  });
});

describe("each change says what it is", () => {
  it("reads as a sentence", () => {
    expect(describeEdit({ kind: "dropColumn", name: "note" })).toBe(
      "Drop note",
    );
    expect(
      describeEdit({ kind: "renameColumn", from: "note", to: "summary" }),
    ).toBe("Rename note to summary");
    expect(
      describeEdit({ kind: "setNullable", name: "note", nullable: false }),
    ).toBe("note becomes not null");
    expect(
      describeEdit({ kind: "setDefault", name: "status", default: "" }),
    ).toBe("Drop the default on status");
  });
});

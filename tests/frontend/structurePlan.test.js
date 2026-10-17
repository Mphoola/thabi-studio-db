import { describe, expect, it } from "vitest";
import {
  describeEdit,
  planWarnings,
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

/**
 * A rename renames the column for everything after it in the same batch.
 * Without this, retyping a column that was just renamed sends the old name to
 * a table that no longer has one, and the second statement fails.
 */
describe("a rename carries through the rest of the batch", () => {
  it("retypes the new name, not the old one", () => {
    const plan = toPlan(structure, [
      { kind: "renameColumn", from: "note", to: "notes" },
      { kind: "setType", name: "note", dataType: "VARCHAR(200)" },
    ]);
    const setType = plan.columns.find((one) => one.setType)?.setType;
    expect(setType).toEqual({ name: "notes", dataType: "VARCHAR(200)" });
    // And the rename is sent first, so the column exists when it is restated.
    expect(Object.keys(plan.columns[0])[0]).toBe("renameColumn");
  });

  it("carries nullability and defaults through too", () => {
    const plan = toPlan(structure, [
      { kind: "renameColumn", from: "note", to: "notes" },
      { kind: "setNullable", name: "note", nullable: false, dataType: "TEXT" },
      { kind: "setDefault", name: "note", default: "'x'" },
    ]);
    expect(plan.columns.find((one) => one.setNullable).setNullable.name).toBe(
      "notes",
    );
    expect(plan.columns.find((one) => one.setDefault).setDefault.name).toBe(
      "notes",
    );
  });

  it("leaves a column that was not renamed alone", () => {
    const plan = toPlan(structure, [
      { kind: "renameColumn", from: "note", to: "notes" },
      { kind: "setType", name: "status", dataType: "TEXT" },
    ]);
    expect(plan.columns.find((one) => one.setType).setType.name).toBe("status");
  });
});

/**
 * A statement that fails halfway has already been shown, so the warning is
 * worth having before it runs rather than after.
 */
describe("a plan says what is likely to be refused", () => {
  const named = { name: "orders", ...structure };

  it("warns about a not-null column added with no default", () => {
    // MySQL refuses exactly this on a table that has rows, and it commits DDL
    // per statement, so the table can be left half changed.
    const warnings = planWarnings(named, {
      columns: [
        {
          addColumn: {
            name: "phaxy",
            dataType: "DECIMAL(10,2)",
            nullable: false,
            default: "",
          },
        },
      ],
    });
    expect(warnings.join(" ")).toMatch(/not null with no default/);
    expect(warnings.join(" ")).toMatch(/refuse/);
  });

  it("says nothing about a nullable column, or one with a default", () => {
    expect(
      planWarnings(named, {
        columns: [
          {
            addColumn: {
              name: "phaxy",
              dataType: "DECIMAL(10,2)",
              nullable: true,
              default: "",
            },
          },
        ],
      }),
    ).toEqual([]);
    expect(
      planWarnings(named, {
        columns: [
          {
            addColumn: {
              name: "phaxy",
              dataType: "DECIMAL(10,2)",
              nullable: false,
              default: "0",
            },
          },
        ],
      }),
    ).toEqual([]);
  });

  it("warns when whole numbers are about to become text", () => {
    const counted = {
      name: "orders",
      columns: [{ name: "hits", dataType: "INT", nullable: false }],
    };
    const warnings = planWarnings(counted, {
      columns: [{ setType: { name: "hits", dataType: "TEXT" } }],
    });
    expect(warnings.join(" ")).toMatch(/will fail the change/);
  });

  it("says nothing about text that is only getting longer", () => {
    // VARCHAR(20) to TEXT fits every value it holds. Warning here would be
    // noise, and noise is why warnings get ignored.
    expect(
      planWarnings(named, {
        columns: [{ setType: { name: "status", dataType: "TEXT" } }],
      }),
    ).toEqual([]);
  });
});

/**
 * Keys are a separate array in the plan because they are separate statements,
 * and getting this wrong sends DDL that refers to a constraint by a name the
 * server does not have.
 */
describe("keys and foreign keys", () => {
  it("sets and drops the primary key", () => {
    const plan = toPlan(structure, [
      { kind: "setPrimaryKey", name: "orders_pkey", columns: ["id"] },
    ]);
    expect(plan.keys).toEqual([
      { setPrimaryKey: { name: "orders_pkey", columns: ["id"] } },
    ]);
    expect(
      toPlan(structure, [{ kind: "dropPrimaryKey", name: "orders_pkey" }]).keys,
    ).toEqual([{ dropPrimaryKey: { name: "orders_pkey" } }]);
  });

  it("defaults the referential actions rather than sending an empty one", () => {
    const plan = toPlan(structure, [
      {
        kind: "addForeignKey",
        name: "child_parent_fk",
        columns: ["parent_id"],
        references: ["public", "parent"],
        referencedColumns: ["id"],
      },
    ]);
    // An empty ON DELETE is refused by all three engines.
    expect(plan.keys[0].addForeignKey.onDelete).toBe("NO ACTION");
    expect(plan.keys[0].addForeignKey.onUpdate).toBe("NO ACTION");
  });

  it("keeps the referential actions that were asked for", () => {
    const plan = toPlan(structure, [
      {
        kind: "addForeignKey",
        name: "child_parent_fk",
        columns: ["parent_id"],
        references: ["public", "parent"],
        referencedColumns: ["id"],
        onDelete: "CASCADE",
        onUpdate: "RESTRICT",
      },
    ]);
    expect(plan.keys[0].addForeignKey.onDelete).toBe("CASCADE");
    expect(plan.keys[0].addForeignKey.onUpdate).toBe("RESTRICT");
  });

  /**
   * A rename has to reach the keys too. A key left naming the old column is
   * DDL against a column that is no longer there, which fails on the server
   * rather than here.
   */
  it("follows a column rename into the keys", () => {
    const plan = toPlan(structure, [
      { kind: "renameColumn", from: "note", to: "notes" },
      {
        kind: "addForeignKey",
        name: "child_notes_fk",
        columns: ["note"],
        references: ["public", "other"],
        referencedColumns: ["id"],
      },
      { kind: "setPrimaryKey", name: "pkey", columns: ["note"] },
    ]);
    expect(
      plan.keys.find((one) => one.addForeignKey).addForeignKey.columns,
    ).toEqual(["notes"]);
    expect(
      plan.keys.find((one) => one.setPrimaryKey).setPrimaryKey.columns,
    ).toEqual(["notes"]);
  });

  it("describes each key change in a sentence", () => {
    expect(
      describeEdit({
        kind: "addForeignKey",
        name: "fk",
        columns: ["parent_id"],
        references: ["public", "parent"],
        referencedColumns: ["id"],
      }),
    ).toBe("Foreign key parent_id → public.parent.id");
    expect(describeEdit({ kind: "dropPrimaryKey", name: "pkey" })).toBe(
      "Drop the primary key",
    );
  });
});

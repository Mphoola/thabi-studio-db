import { describe, expect, it } from "vitest";
import {
  upsertProfile,
  colourDotClass,
  groupConnections,
  groupEdgeClass,
  isCollapsed,
  isColour,
  reorderWithin,
} from "../../src/lib/connectionGroups.js";

const profile = (over) => ({
  id: over.name,
  name: over.name,
  environment: "local",
  groupName: "",
  colour: "",
  ...over,
});

describe("groupConnections", () => {
  it("puts user groups first and environments after, production first", () => {
    const groups = groupConnections([
      profile({ name: "Local pg", environment: "local" }),
      profile({ name: "Prod", environment: "production" }),
      profile({ name: "Work laptop", groupName: "Work" }),
      profile({ name: "Work db", groupName: "work" }),
    ]);

    expect(groups.map((g) => g.label)).toEqual(["Work", "Production", "Local"]);
    // Case differences are the same group, keeping the order Rust gave.
    expect(groups[0].profiles.map((p) => p.name)).toEqual([
      "Work laptop",
      "Work db",
    ]);
    expect(groups[1].user).toBe(false);
    expect(groups[0].user).toBe(true);
  });

  it("keeps the search filter and leaves out empty sections", () => {
    const groups = groupConnections(
      [
        profile({ name: "Work laptop", groupName: "Work" }),
        profile({ name: "Prod", environment: "production" }),
      ],
      (p) => p.name.includes("Work"),
    );
    expect(groups.map((g) => g.label)).toEqual(["Work"]);
  });

  it("still groups an unknown environment", () => {
    const groups = groupConnections([
      profile({ name: "Odd", environment: "sandbox" }),
    ]);
    expect(groups.map((g) => g.label)).toEqual(["Unknown"]);
  });

  it("marks groups for styling", () => {
    const [user, environment] = groupConnections([
      profile({ name: "Work laptop", groupName: "Work" }),
      profile({ name: "Prod", environment: "production" }),
    ]);
    expect(groupEdgeClass(user)).toBe("border-l-tag-gray");
    expect(groupEdgeClass(environment)).toBe("border-l-tag-red");
  });
});

describe("reorderWithin", () => {
  it("moves a dragged profile onto the target's place", () => {
    expect(reorderWithin(["a", "b", "c"], "a", "c")).toEqual(["b", "a", "c"]);
    expect(reorderWithin(["a", "b", "c"], "c", "a")).toEqual(["c", "a", "b"]);
    expect(reorderWithin(["a", "b"], "b", "a")).toEqual(["b", "a"]);
  });

  it("leaves the order alone for unknown or identical rows", () => {
    expect(reorderWithin(["a", "b"], "a", "a")).toEqual(["a", "b"]);
    expect(reorderWithin(["a", "b"], "a", "z")).toEqual(["a", "b"]);
    expect(reorderWithin(["a", "b"], "z", "a")).toEqual(["a", "b"]);
  });
});

describe("colours", () => {
  it("uses the custom colour when set, the environment's otherwise", () => {
    expect(colourDotClass(profile({ environment: "production" }))).toBe(
      "bg-tag-red",
    );
    expect(
      colourDotClass(profile({ environment: "production", colour: "purple" })),
    ).toBe("bg-tag-purple");
  });

  it("only accepts known tokens", () => {
    expect(isColour("")).toBe(true);
    expect(isColour("purple")).toBe(true);
    expect(isColour("chartreuse")).toBe(false);
  });
});

describe("isCollapsed", () => {
  it("folds user groups away but never environment sections", () => {
    const user = { user: true, key: "Work" };
    const environment = { user: false, key: "production" };
    expect(isCollapsed(["Work"], user)).toBe(true);
    expect(isCollapsed([], user)).toBe(false);
    expect(isCollapsed(["production"], environment)).toBe(false);
  });
});

describe("upsertProfile", () => {
  const a = {
    id: "a",
    name: "Alpha",
    environment: "local",
    groupName: "",
    sortOrder: 0,
  };
  it("adds a new connection, so it shows up after Save", () => {
    const b = {
      id: "b",
      name: "Beta",
      environment: "local",
      groupName: "",
      sortOrder: 1,
    };
    expect(upsertProfile([a], b).map((p) => p.id)).toEqual(["a", "b"]);
  });
  it("replaces an edited connection instead of adding it twice", () => {
    const renamed = { ...a, name: "Alpha 2" };
    const out = upsertProfile([a], renamed);
    expect(out).toHaveLength(1);
    expect(out[0].name).toBe("Alpha 2");
  });
});

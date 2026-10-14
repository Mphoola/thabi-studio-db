import { describe, expect, it } from "vitest";
import { fuzzyFilter, fuzzyScore } from "../../src/lib/fuzzy.js";

describe("fuzzyScore", () => {
  it("requires characters in order", () => {
    expect(fuzzyScore("tsb", "Toggle Sidebar")).toBeGreaterThan(0);
    expect(fuzzyScore("bst", "Toggle Sidebar")).toBe(-1);
  });

  it("is case-insensitive", () => {
    expect(fuzzyScore("SET", "Settings…")).toBeGreaterThan(0);
  });
});

describe("fuzzyFilter", () => {
  const items = [
    "Toggle Sidebar",
    "Settings…",
    "Keyboard Shortcuts",
    "About Thabi Studio",
  ];

  it("returns everything for an empty query", () => {
    expect(fuzzyFilter("  ", items, (x) => x)).toEqual(items);
  });

  it("ranks word-start and consecutive matches first", () => {
    expect(fuzzyFilter("set", items, (x) => x)[0]).toBe("Settings…");
    expect(fuzzyFilter("ks", items, (x) => x)[0]).toBe("Keyboard Shortcuts");
  });

  it("drops non-matches", () => {
    expect(fuzzyFilter("zzz", items, (x) => x)).toEqual([]);
  });
});

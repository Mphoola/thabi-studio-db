import { describe, expect, it } from "vitest";
import {
  isObjectTab,
  tabIcon,
  tabTitle,
} from "../../src/lib/tabPresentation.js";

/**
 * A tab is not always about one database object. These are the shapes the tab
 * bar actually gets, and every one of them has to survive being drawn.
 */
const query = { id: "1", type: "query", title: "Query 1" };
const table = {
  id: "2",
  type: "object",
  title: "orders",
  node: { kind: "table", name: "orders" },
};
const view = {
  id: "3",
  type: "object",
  title: "recent_orders",
  node: { kind: "view", name: "recent_orders" },
};
const diagram = {
  id: "4",
  type: "diagram",
  title: "shop diagram",
  database: "shop",
  schema: "public",
};

describe("tab icons", () => {
  it("draws a query tab", () => {
    expect(tabIcon(query)).toBeTruthy();
  });

  it("draws a table differently from a view", () => {
    expect(tabIcon(table)).not.toBe(tabIcon(view));
  });

  /**
   * The bug this module exists for: a diagram tab has no `node`, and reading
   * `tab.node.kind` on one threw the whole tab bar over.
   */
  it("draws a diagram tab, which has no node at all", () => {
    expect(() => tabIcon(diagram)).not.toThrow();
    expect(tabIcon(diagram)).toBeTruthy();
    expect(diagram.node).toBeUndefined();
  });

  it("survives a tab kind nobody planned for", () => {
    expect(() => tabIcon({ id: "5", type: "something-new" })).not.toThrow();
    expect(() => tabIcon({ id: "6" })).not.toThrow();
    expect(() => tabIcon(null)).not.toThrow();
    expect(() => tabIcon(undefined)).not.toThrow();
  });

  it("survives an object tab whose node went missing", () => {
    expect(() => tabIcon({ id: "7", type: "object" })).not.toThrow();
    expect(() =>
      tabIcon({ id: "8", type: "object", node: null }),
    ).not.toThrow();
  });
});

describe("what counts as an object tab", () => {
  it("only the explorer ones", () => {
    expect(isObjectTab(table)).toBe(true);
    expect(isObjectTab(diagram)).toBe(false);
    expect(isObjectTab(query)).toBe(false);
  });
});

describe("tab titles", () => {
  it("shows the title a tab has", () => {
    expect(tabTitle(diagram)).toBe("shop diagram");
  });

  it("says something rather than rendering nothing", () => {
    expect(tabTitle({ title: "" })).toBe("Untitled");
    expect(tabTitle({})).toBe("Untitled");
  });
});

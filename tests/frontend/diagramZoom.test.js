import { describe, expect, it } from "vitest";
import {
  openingView,
  zoomAround,
} from "../../src/composables/useDiagramZoom.js";
import {
  elbowRoute,
  gridBelow,
  moveNode,
  splitIsolated,
} from "../../src/lib/erLayout.js";

describe("zooming", () => {
  it("keeps the point under the pointer where it was", () => {
    const offset = { x: 40, y: 30 };
    const at = { x: 400, y: 300 };
    const next = zoomAround(offset, at, 1, 2);
    // The diagram point under `at` before and after is the same.
    const before = { x: (at.x - offset.x) / 1, y: (at.y - offset.y) / 1 };
    const after = { x: (at.x - next.x) / 2, y: (at.y - next.y) / 2 };
    expect(after).toEqual(before);
  });
});

describe("opening a diagram", () => {
  const box = { width: 1000, height: 700 };

  it("centres a small diagram at full size", () => {
    const view = openingView(box, { width: 400, height: 300 });
    expect(view.zoom).toBe(1);
    expect(view.offset).toEqual({ x: 300, y: 200 });
  });

  it("shrinks a big one to fit", () => {
    const view = openingView(box, { width: 3000, height: 1000 });
    expect(view.zoom).toBeLessThan(1);
    expect(3000 * view.zoom).toBeLessThanOrEqual(box.width);
  });
});

describe("tables with no relationships", () => {
  const node = (id) => ({ id, width: 220, height: 100 });
  const graph = {
    children: [node("a"), node("b"), node("lonely"), node("alone")],
    edges: [{ id: "k", sources: ["a:x:out"], targets: ["b:id:in"] }],
  };

  it("are kept out of the related diagram", () => {
    const { connected, isolated } = splitIsolated(graph);
    expect(connected.children.map((n) => n.id)).toEqual(["a", "b"]);
    expect(isolated.map((n) => n.id)).toEqual(["lonely", "alone"]);
  });

  it("go in a grid beneath it, under a caption", () => {
    const { isolated } = splitIsolated(graph);
    const grid = gridBelow(isolated, 500, 1200);
    expect(grid.children.every((n) => n.y > 500)).toBe(true);
    expect(grid.children[1].x).toBeGreaterThan(grid.children[0].x);
    expect(grid.caption).toMatchObject({ count: 2 });
  });
});

describe("dragging a table", () => {
  const placed = {
    nodes: {
      a: {
        id: "a",
        x: 0,
        y: 0,
        width: 200,
        height: 80,
        ports: { "a:fk:out": { x: 200, y: 50 } },
      },
      b: {
        id: "b",
        x: 400,
        y: 0,
        width: 200,
        height: 80,
        ports: { "b:id:in": { x: 0, y: 50 } },
      },
      c: { id: "c", x: 0, y: 300, width: 200, height: 80, ports: {} },
    },
    lines: [
      {
        id: "k",
        source: "a:fk:out",
        target: "b:id:in",
        fromTable: "a",
        toTable: "b",
        points: [
          { x: 200, y: 50 },
          { x: 400, y: 50 },
        ],
      },
    ],
  };

  it("moves the table and re-routes its lines in right angles", () => {
    const moved = moveNode(placed, "b", 500, 200);
    expect(moved.nodes.b).toMatchObject({ x: 500, y: 200 });
    const points = moved.lines[0].points;
    expect(points[0]).toEqual({ x: 200, y: 50 });
    expect(points.at(-1)).toEqual({ x: 500, y: 250 });
    for (let i = 1; i < points.length; i += 1)
      expect(
        points[i].x === points[i - 1].x || points[i].y === points[i - 1].y,
      ).toBe(true);
  });

  it("goes around when the target ends up behind", () => {
    expect(elbowRoute({ x: 200, y: 50 }, { x: 100, y: 300 })).toHaveLength(6);
  });

  it("leaves other tables' lines alone", () => {
    const moved = moveNode(placed, "c", 50, 50);
    expect(moved.lines[0]).toBe(placed.lines[0]);
  });
});

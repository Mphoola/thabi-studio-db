import { describe, expect, it } from "vitest";
import {
  HEADER_HEIGHT,
  ROW_HEIGHT,
  buildGraph,
  extentOf,
  layoutOptions,
  nodeHeight,
  nodeWidth,
  plainGraph,
  positionsFrom,
  roundedPath,
} from "../../src/lib/erLayout.js";

/** Two tables and the line between them: the smallest thing worth drawing. */
const diagram = {
  tables: [
    {
      name: "orders",
      columns: [
        { name: "id", dataType: "bigint", nullable: false, primaryKey: true },
        {
          name: "customer_id",
          dataType: "int",
          nullable: false,
          primaryKey: false,
        },
      ],
    },
    {
      name: "customers",
      columns: [
        { name: "id", dataType: "bigint", nullable: false, primaryKey: true },
        { name: "email", dataType: "text", nullable: true, primaryKey: false },
      ],
    },
  ],
  foreignKeys: [
    {
      name: "orders_customer_fk",
      fromTable: "orders",
      fromColumns: ["customer_id"],
      toTable: "customers",
      toColumns: ["id"],
    },
  ],
};

describe("a table box is sized before anything is arranged", () => {
  it("grows with its columns", () => {
    const two = nodeHeight({ columns: [{}, {}] });
    const four = nodeHeight({ columns: [{}, {}, {}, {}] });
    expect(four - two).toBe(2 * ROW_HEIGHT);
    expect(two).toBeGreaterThan(HEADER_HEIGHT);
  });

  it("is wide enough for its longest name and type side by side", () => {
    const narrow = nodeWidth({
      name: "t",
      columns: [{ name: "id", dataType: "int" }],
    });
    const wide = nodeWidth({
      name: "t",
      columns: [
        { name: "a_rather_long_column_name", dataType: "varchar(255)" },
      ],
    });
    expect(narrow).toBeGreaterThanOrEqual(200);
    expect(wide).toBeGreaterThan(narrow);
  });

  it("adds a row for the columns it leaves out, or to show fewer again", () => {
    const columns = Array.from({ length: 20 }, () => ({}));
    const capped = nodeHeight({ columns }, 14);
    expect(capped - nodeHeight({ columns: columns.slice(0, 14) }, 14)).toBe(
      ROW_HEIGHT,
    );
    // Expanded, the last row is "Show fewer", so the box still has room for it.
    expect(nodeHeight({ columns }, Infinity) - capped).toBe(6 * ROW_HEIGHT);
  });
});

describe("the graph elk is asked to arrange", () => {
  it("gives every column a port, so a line lands on the column it belongs to", () => {
    const graph = buildGraph(diagram);
    const ports = graph.children.flatMap((node) =>
      node.ports.map((port) => port.id),
    );
    expect(ports).toContain("orders:id:out");
    expect(ports).toContain("orders:customer_id:in");
  });

  it("pins each port to its column's row, as ports and not as boxes inside the table", () => {
    // Handed over as children, elk shrank every table around them to a sliver
    // and drew the schema as an unreadable grid.
    const orders = buildGraph(diagram).children[0];
    expect(orders.children).toBeUndefined();
    expect(orders.layoutOptions["elk.portConstraints"]).toBe("FIXED_POS");
    const out = orders.ports.find((p) => p.id === "orders:customer_id:out");
    expect(out.x).toBe(orders.width);
    expect(out.y).toBe(HEADER_HEIGHT + ROW_HEIGHT + ROW_HEIGHT / 2);
  });

  it("hands elk children as a list, which is the shape it can read", () => {
    // As a keyed object, elk cannot find the ports its own edges point at and
    // refuses the graph. Nothing about the error says so.
    const graph = buildGraph(diagram);
    expect(Array.isArray(graph.children)).toBe(true);
  });

  it("joins the foreign key's columns", () => {
    const graph = buildGraph(diagram);
    expect(graph.edges).toEqual([
      {
        id: "orders_customer_fk",
        sources: ["orders:customer_id:out"],
        targets: ["customers:id:in"],
      },
    ]);
  });

  it("draws a composite key once, from its first column", () => {
    const graph = buildGraph({
      ...diagram,
      foreignKeys: [
        {
          name: "composite",
          fromTable: "orders",
          fromColumns: ["customer_id", "id"],
          toTable: "customers",
          toColumns: ["id", "tenant_id"],
        },
      ],
    });
    // Two lines for one key is the clutter this diagram exists to avoid.
    expect(graph.edges).toHaveLength(1);
  });

  it("asks for layered layout with room between the layers", () => {
    const options = layoutOptions();
    expect(options["elk.algorithm"]).toBe("layered");
    expect(options["elk.layered.spacing.nodeNodeBetweenLayers"]).toBeTruthy();
  });

  it("survives a schema with no keys at all", () => {
    expect(
      buildGraph({ tables: diagram.tables, foreignKeys: [] }).edges,
    ).toEqual([]);
  });

  it("skips a key pointing at a table it was not given", () => {
    const graph = buildGraph({
      tables: [diagram.tables[0]],
      foreignKeys: [diagram.foreignKeys[0]],
    });
    // Drawing a line into nothing is worse than not drawing the line.
    expect(graph.edges).toEqual([]);
  });

  // A generous ceiling, deliberately. This asserts that elk can place a
  // hundred ported tables at all; asserting a tight time here measures the
  // machine the test happens to run on — it failed at 6 s while a dev server
  // and three databases were also running, and passed in under a second when
  // they were not. How long the diagram takes to appear is a question for a
  // measurement on a real window, not for a unit test.
  it("arranges a hundred tables through elk", { timeout: 20_000 }, async () => {
    const ELK = (await import("elkjs/lib/elk.bundled.js")).default;
    const many = {
      tables: Array.from({ length: 100 }, (_, index) => ({
        name: `table_${index}`,
        columns: Array.from({ length: 12 }, (_, column) => ({
          name: `column_${column}`,
        })),
      })),
      foreignKeys: Array.from({ length: 99 }, (_, index) => ({
        name: `fk_${index}`,
        fromTable: `table_${index + 1}`,
        fromColumns: ["column_1"],
        toTable: `table_${index}`,
        toColumns: ["column_0"],
      })),
    };
    // The criterion this phase has to meet, run rather than assumed.
    const laid = await new ELK().layout(buildGraph(many));
    expect(laid.children).toHaveLength(100);
    // Boxes keep the size they asked for, and lines come back routed.
    const asked = buildGraph(many).children[0];
    expect(laid.children[0].width).toBe(asked.width);
    expect(laid.edges[0].sections?.[0]?.startPoint).toBeTruthy();
    expect(laid.children.every((node) => typeof node.x === "number")).toBe(
      true,
    );
  });

  it("skips a key whose columns the table does not have", () => {
    const graph = buildGraph({
      ...diagram,
      foreignKeys: [
        {
          name: "ghost",
          fromTable: "orders",
          fromColumns: ["never_a_column"],
          toTable: "customers",
          toColumns: ["id"],
        },
      ],
    });
    expect(graph.edges[0].sources).toEqual(["orders:never_a_column:out"]);
  });
});

describe("reading elk's answer back", () => {
  const laidOut = {
    children: [
      {
        id: "orders",
        x: 10,
        y: 20,
        width: 220,
        height: 80,
        ports: [
          { id: "orders:customer_id:out", x: 220, y: 40, width: 8, height: 8 },
        ],
      },
      {
        id: "customers",
        x: 400,
        y: 20,
        width: 220,
        height: 80,
        ports: [{ id: "customers:id:in", x: 0, y: 40, width: 8, height: 8 }],
      },
    ],
    edges: [
      {
        id: "orders_customer_fk",
        sources: ["orders:customer_id:out"],
        targets: ["customers:id:in"],
      },
    ],
  };

  it("places a line at the port, not the middle of the box", () => {
    const { lines } = positionsFrom(laidOut);
    expect(lines).toHaveLength(1);
    // Port x is relative to its own table, so the table's position is added.
    expect(lines[0].from).toEqual({ x: 234, y: 64 });
    expect(lines[0].to).toEqual({ x: 404, y: 64 });
    // Needed to highlight a table's keys on hover.
    expect(lines[0].fromTable).toBe("orders");
    expect(lines[0].toTable).toBe("customers");
  });

  it("drops an edge whose port came back missing", () => {
    const { lines } = positionsFrom({
      ...laidOut,
      edges: [
        {
          id: "gone",
          sources: ["orders:nope:out"],
          targets: ["customers:id:in"],
        },
      ],
    });
    expect(lines).toEqual([]);
  });

  it("measures how far the drawing reaches", () => {
    const { nodes } = positionsFrom(laidOut);
    expect(extentOf(nodes)).toEqual({ width: 620, height: 100 });
  });
});

describe("a routed line", () => {
  it("follows elk's route when there is one", () => {
    const { lines } = positionsFrom({
      children: [
        {
          id: "a",
          x: 0,
          y: 0,
          width: 100,
          height: 50,
          ports: [{ id: "a:x:out", x: 100, y: 20 }],
        },
        {
          id: "b",
          x: 300,
          y: 200,
          width: 100,
          height: 50,
          ports: [{ id: "b:y:in", x: 0, y: 20 }],
        },
      ],
      edges: [
        {
          id: "k",
          sources: ["a:x:out"],
          targets: ["b:y:in"],
          sections: [
            {
              startPoint: { x: 100, y: 20 },
              bendPoints: [
                { x: 200, y: 20 },
                { x: 200, y: 220 },
              ],
              endPoint: { x: 300, y: 220 },
            },
          ],
        },
      ],
    });
    expect(lines[0].points).toHaveLength(4);
  });

  it("rounds its corners", () => {
    const d = roundedPath([
      { x: 0, y: 0 },
      { x: 100, y: 0 },
      { x: 100, y: 100 },
    ]);
    expect(d.startsWith("M0,0")).toBe(true);
    expect(d).toContain("Q100,0");
    expect(d.endsWith("L100,100")).toBe(true);
  });
});

describe("the first paint, before elk has answered", () => {
  it("is arranged enough to be looked at", () => {
    const { nodes, lines } = plainGraph(buildGraph(diagram));
    expect(Object.keys(nodes)).toHaveLength(2);
    // Side by side, not overlapping: a blank panel helps nobody.
    expect(nodes.customers.x).toBeGreaterThanOrEqual(
      nodes.orders.x + nodes.orders.width,
    );
    expect(lines).toHaveLength(1);
  });
});

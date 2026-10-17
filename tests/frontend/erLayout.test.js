import { describe, expect, it } from "vitest";
import {
  HEADER_HEIGHT,
  NODE_WIDTH,
  buildGraph,
  extentOf,
  layoutOptions,
  nodeHeight,
  plainGraph,
  positionsFrom,
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
    expect(four - two).toBe(40);
    expect(two).toBeGreaterThan(HEADER_HEIGHT);
  });

  it("has a width worth drawing a column name in", () => {
    expect(NODE_WIDTH).toBeGreaterThanOrEqual(180);
  });
});

describe("the graph elk is asked to arrange", () => {
  it("gives every column a port, so a line lands on the column it belongs to", () => {
    const graph = buildGraph(diagram);
    const ports = graph.children.flatMap((node) =>
      node.children.map((port) => port.id),
    );
    expect(ports).toContain("orders:id:out");
    expect(ports).toContain("orders:customer_id:in");
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

  it("arranges a hundred tables through elk", async () => {
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
        children: [
          { id: "orders:customer_id:out", x: 220, y: 40, width: 8, height: 8 },
        ],
      },
      {
        id: "customers",
        x: 400,
        y: 20,
        width: 220,
        height: 80,
        children: [{ id: "customers:id:in", x: 0, y: 40, width: 8, height: 8 }],
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

describe("the first paint, before elk has answered", () => {
  it("is arranged enough to be looked at", () => {
    const { nodes, lines } = plainGraph(buildGraph(diagram));
    expect(Object.keys(nodes)).toHaveLength(2);
    // Stacked, not overlapping: a blank panel helps nobody.
    expect(nodes.customers.y).toBeGreaterThan(nodes.orders.y);
    expect(lines).toHaveLength(1);
  });
});

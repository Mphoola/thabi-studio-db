/**
 * Turns tables and foreign keys into positioned boxes and lines, using elkjs.
 *
 * Kept apart from the component so the geometry can be tested without a DOM:
 * the layout library is injected, and the only thing these tests need is
 * something that records what it was asked for and returns a fixed answer.
 *
 * ELK is loaded lazily. The editor is the app, not the diagram, and a hundred
 * tables of layout code should not be paid for by someone typing SQL.
 */

/** Column widths and row heights are fixed, so a box is known before layout. */
export const NODE_WIDTH = 220;
export const HEADER_HEIGHT = 30;
export const ROW_HEIGHT = 20;
export const PADDING = 10;

/** The height a table box needs for its header and rows. */
export function nodeHeight(table) {
  return HEADER_HEIGHT + table.columns.length * ROW_HEIGHT + PADDING;
}

/**
 * The elkjs layout options.
 *
 * Orthogonal edges with no bends drawn back through the boxes, and spacing
 * generous enough that a hundred tables are legible rather than merely present.
 */
export function layoutOptions() {
  return {
    "elk.algorithm": "layered",
    "elk.direction": "RIGHT",
    "elk.layered.spacing.nodeNodeBetweenLayers": "90",
    "elk.spacing.nodeNode": "36",
    "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX",
    // Arrows between boxes read better than elbows routed through them.
    "elk.edgeRouting": "ORTHOGONAL",
    "elk.layered.crossingMinimization.semiInteractive": "false",
  };
}

/**
 * The graph elk is asked to arrange.
 *
 * Ports are what let a line join the actual column it belongs to rather than
 * the middle of the box; without them a diagram of linked tables is a diagram
 * of boxes with lines to nowhere.
 */
export function buildGraph(diagram) {
  const children = {};
  const edges = [];

  for (const table of diagram.tables ?? []) {
    // Ports are children of their node, each tagged in or out. Declaring them
    // as a side map is not the shape elk reads, and it says so by refusing to
    // resolve an edge to a port it cannot find.
    const ports = [];
    for (const column of table.columns) {
      for (const side of ["in", "out"]) {
        ports.push({
          id: `${table.name}:${column.name}:${side}`,
          port: side,
          width: 8,
          height: 8,
        });
      }
    }
    children[table.name] = {
      id: table.name,
      width: NODE_WIDTH,
      height: nodeHeight(table),
      children: ports,
    };
  }

  for (const key of diagram.foreignKeys ?? diagram.foreign_keys ?? []) {
    // A composite key is drawn once, from its first column: a hundred tables
    // of overlapping lines is the thing this is meant to avoid.
    const from = key.fromColumns?.[0] ?? key.from_columns?.[0];
    const to = key.toColumns?.[0] ?? key.to_columns?.[0];
    if (!from || !to) continue;
    if (!children[key.fromTable] || !children[key.toTable]) continue;
    edges.push({
      id: key.name,
      sources: [`${key.fromTable}:${from}:out`],
      targets: [`${key.toTable}:${to}:in`],
    });
  }

  // An array, not a keyed object: elk reads children positionally and a map
  // makes it unable to find the ports its own edges point at.
  return {
    id: "root",
    layoutOptions: layoutOptions(),
    children: Object.values(children),
    edges,
  };
}

/**
 * Reads elk's answer back into plain boxes the template can draw.
 *
 * Ports come back nested under the node they belong to, so a line's endpoint
 * is the port's position offset by the position of its table.
 */
export function positionsFrom(elkGraph) {
  const nodes = {};
  const lines = [];

  for (const child of elkGraph.children ?? []) {
    const ports = {};
    for (const port of child.children ?? []) {
      ports[port.id] = port;
    }
    nodes[child.id] = {
      id: child.id,
      x: child.x ?? 0,
      y: child.y ?? 0,
      width: child.width ?? NODE_WIDTH,
      height: child.height ?? nodeHeight({ columns: [] }),
      ports,
    };
  }

  for (const edge of elkGraph.edges ?? []) {
    const source = edge.sources?.[0];
    const target = edge.targets?.[0];
    if (!source || !target) continue;
    const from = portOwner(nodes, source);
    const to = portOwner(nodes, target);
    if (!from || !to) continue;
    lines.push({
      id: edge.id,
      from: {
        x: from.node.x + from.port.x + from.port.width / 2,
        y: from.node.y + from.port.y + from.port.height / 2,
      },
      to: {
        x: to.node.x + to.port.x + to.port.width / 2,
        y: to.node.y + to.port.y + to.port.height / 2,
      },
    });
  }

  return { nodes, lines };
}

function portOwner(nodes, portId) {
  const [nodeId, ...rest] = portId.split(":");
  const node = nodes[nodeId];
  if (!node) return null;
  const port = node.ports[portId];
  if (!port) return null;
  void rest;
  return { node, port };
}

/** How far the drawn area reaches, for fitting the diagram to the window. */
export function extentOf(nodes) {
  let width = 0;
  let height = 0;
  for (const node of Object.values(nodes)) {
    width = Math.max(width, node.x + node.width);
    height = Math.max(height, node.y + node.height);
  }
  return { width, height };
}

/**
 * A stand-in for elkjs, for tests and for the first paint.
 *
 * Drawing the real layout takes a moment on a large schema; an unarranged but
 * correct picture is better than a blank panel while it works out an answer.
 */
export function plainGraph(graph) {
  let y = 0;
  const children = Object.values(graph.children).map((node) => {
    const placed = {
      id: node.id,
      x: 0,
      y,
      width: node.width,
      height: node.height,
      // Ports come back from elk in node order, so their y is a fraction of
      // the box; this puts them all on the right-hand edge until it answers.
      children: (node.children ?? []).map((port) => ({
        ...port,
        x: node.width,
        y: HEADER_HEIGHT / 2,
      })),
    };
    y += node.height + 48;
    return placed;
  });
  return positionsFrom({ children, edges: graph.edges });
}

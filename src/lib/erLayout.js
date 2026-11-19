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

export const HEADER_HEIGHT = 34;
export const ROW_HEIGHT = 22;
export const PADDING = 6;
/** Columns drawn per box until someone asks for the rest. */
export const ROW_LIMIT = 14;
const MIN_WIDTH = 200;
const MAX_WIDTH = 380;

/**
 * Rough text widths at the sizes the canvas draws (12px names, 11px mono
 * types). Measuring in the DOM would tie the layout to a browser; a box a
 * little wide is fine, one too narrow overlaps its own text.
 */
const NAME_CHAR = 6.8;
const TYPE_CHAR = 6.7;
const TITLE_CHAR = 7.6;

/** A box wide enough for its longest name and type side by side. */
export function nodeWidth(table) {
  let widest = (table.name?.length ?? 0) * TITLE_CHAR + 70;
  for (const column of table.columns ?? []) {
    const row =
      (column.name?.length ?? 0) * NAME_CHAR +
      (column.dataType?.length ?? 0) * TYPE_CHAR +
      // Key badge, gap between name and type, both paddings.
      64;
    widest = Math.max(widest, row);
  }
  return Math.round(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, widest)));
}

/**
 * Rows a box draws, how many it leaves out, and whether it has a last row
 * for "more columns" or "show fewer".
 */
export function visibleRows(table, limit = ROW_LIMIT) {
  const columns = table.columns ?? [];
  const shown = Math.min(columns.length, limit);
  const hidden = columns.length - shown;
  return { shown, hidden, toggle: hidden > 0 || columns.length > ROW_LIMIT };
}

/** The height a table box needs for its header, rows and toggle row. */
export function nodeHeight(table, limit = ROW_LIMIT) {
  const { shown, toggle } = visibleRows(table, limit);
  return HEADER_HEIGHT + (shown + (toggle ? 1 : 0)) * ROW_HEIGHT + PADDING;
}

/** Middle of a column's row, or of the "more" row for a hidden column. */
function rowCenter(index, shown) {
  return HEADER_HEIGHT + Math.min(index, shown) * ROW_HEIGHT + ROW_HEIGHT / 2;
}

/**
 * The elkjs layout options.
 *
 * Left to right, with lines routed around the boxes in right angles and room
 * between layers for them to turn.
 */
export function layoutOptions() {
  return {
    "elk.algorithm": "layered",
    "elk.direction": "RIGHT",
    "elk.edgeRouting": "ORTHOGONAL",
    "elk.layered.spacing.nodeNodeBetweenLayers": "110",
    "elk.layered.spacing.edgeNodeBetweenLayers": "24",
    "elk.layered.spacing.edgeEdgeBetweenLayers": "14",
    "elk.spacing.nodeNode": "48",
    "elk.spacing.edgeNode": "24",
    "elk.spacing.edgeEdge": "14",
    "elk.spacing.componentComponent": "80",
    "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX",
    "elk.separateConnectedComponents": "true",
  };
}

/**
 * The graph elk is asked to arrange.
 *
 * Every column has a port on each side, fixed at the middle of its row, so a
 * line leaves from the foreign key's own row and arrives at the key it points
 * to. Ports go in a node's `ports`: given as `children`, elk reads them as
 * boxes inside the table, shrinks the table around them and routes nothing.
 *
 * @param diagram { tables, foreignKeys }
 * @param limitFor table name -> rows drawn (the cap, or Infinity when expanded)
 */
export function buildGraph(diagram, limitFor = () => ROW_LIMIT) {
  const children = [];
  const known = new Set();
  const edges = [];

  for (const table of diagram.tables ?? []) {
    const limit = limitFor(table.name);
    const width = nodeWidth(table);
    const { shown } = visibleRows(table, limit);
    const ports = [];
    (table.columns ?? []).forEach((column, index) => {
      const y = rowCenter(index, shown);
      ports.push(
        {
          id: `${table.name}:${column.name}:in`,
          x: 0,
          y,
          width: 0,
          height: 0,
          layoutOptions: { "elk.port.side": "WEST" },
        },
        {
          id: `${table.name}:${column.name}:out`,
          x: width,
          y,
          width: 0,
          height: 0,
          layoutOptions: { "elk.port.side": "EAST" },
        },
      );
    });
    known.add(table.name);
    children.push({
      id: table.name,
      width,
      height: nodeHeight(table, limit),
      ports,
      layoutOptions: { "elk.portConstraints": "FIXED_POS" },
    });
  }

  for (const key of diagram.foreignKeys ?? diagram.foreign_keys ?? []) {
    // A composite key is drawn once, from its first column: a hundred tables
    // of overlapping lines is the thing this is meant to avoid.
    const from = key.fromColumns?.[0] ?? key.from_columns?.[0];
    const to = key.toColumns?.[0] ?? key.to_columns?.[0];
    if (!from || !to) continue;
    if (!known.has(key.fromTable) || !known.has(key.toTable)) continue;
    edges.push({
      id: key.name,
      sources: [`${key.fromTable}:${from}:out`],
      targets: [`${key.toTable}:${to}:in`],
    });
  }

  return { id: "root", layoutOptions: layoutOptions(), children, edges };
}

/**
 * Reads elk's answer back into plain boxes and lines the template can draw.
 *
 * A line is the route elk worked out (start, bends, end), in diagram
 * coordinates. Without a route it is a straight line between the two ports.
 */
export function positionsFrom(elkGraph) {
  const nodes = {};
  const lines = [];

  for (const child of elkGraph.children ?? []) {
    const ports = {};
    for (const port of child.ports ?? []) ports[port.id] = port;
    nodes[child.id] = {
      id: child.id,
      x: child.x ?? 0,
      y: child.y ?? 0,
      width: child.width ?? MIN_WIDTH,
      height: child.height ?? HEADER_HEIGHT + PADDING,
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
    const start = portPoint(from);
    const end = portPoint(to);
    const section = edge.sections?.[0];
    const points = section
      ? [
          section.startPoint,
          ...(section.bendPoints ?? []),
          section.endPoint,
        ].map(({ x, y }) => ({ x, y }))
      : [start, end];
    lines.push({
      id: edge.id,
      // The ports it joins, so a line can be re-routed when a table moves.
      source,
      target,
      // Which tables it joins, so hovering one can light its keys and dim the
      // rest.
      fromTable: from.node.id,
      toTable: to.node.id,
      from: points[0],
      to: points.at(-1),
      points,
    });
  }

  return { nodes, lines };
}

function portOwner(nodes, portId) {
  const node = nodes[portId.split(":")[0]];
  const port = node?.ports[portId];
  return port ? { node, port } : null;
}

function portPoint({ node, port }) {
  return {
    x: node.x + port.x + (port.width ?? 0) / 2,
    y: node.y + port.y + (port.height ?? 0) / 2,
  };
}

/**
 * A right-angled route from a foreign key's port (right side of its table) to
 * the key's port (left side of the other), for a line whose table was dragged
 * away from where elk routed it.
 */
export function elbowRoute(from, to) {
  if (to.x - from.x >= 48) {
    const middle = Math.round((from.x + to.x) / 2);
    return [from, { x: middle, y: from.y }, { x: middle, y: to.y }, to];
  }
  // The target is behind: step out, cross over, step back in.
  const out = from.x + 24;
  const back = to.x - 24;
  const across = Math.round((from.y + to.y) / 2);
  return [
    from,
    { x: out, y: from.y },
    { x: out, y: across },
    { x: back, y: across },
    { x: back, y: to.y },
    to,
  ];
}

/**
 * Moves a table to (x, y) and re-routes the lines that touch it; the rest
 * keep elk's routes.
 */
export function moveNode({ nodes, lines }, id, x, y) {
  const node = nodes[id];
  if (!node) return { nodes, lines };
  const next = { ...nodes, [id]: { ...node, x, y } };
  const point = (portId) => {
    const owner = portOwner(next, portId);
    return owner ? portPoint(owner) : null;
  };
  return {
    nodes: next,
    lines: lines.map((line) => {
      if (line.fromTable !== id && line.toTable !== id) return line;
      const from = point(line.source);
      const to = point(line.target);
      if (!from || !to) return line;
      const points = elbowRoute(from, to);
      return { ...line, from, to, points };
    }),
  };
}

/**
 * An SVG path through a line's points, with its corners rounded so a route of
 * right angles reads as one line rather than a staircase.
 */
export function roundedPath(points, radius = 8) {
  if (!points?.length) return "";
  let d = `M${points[0].x},${points[0].y}`;
  for (let i = 1; i < points.length - 1; i += 1) {
    const [a, b, c] = [points[i - 1], points[i], points[i + 1]];
    const r = Math.min(
      radius,
      Math.hypot(b.x - a.x, b.y - a.y) / 2,
      Math.hypot(c.x - b.x, c.y - b.y) / 2,
    );
    const towards = (p, q) => {
      const length = Math.hypot(q.x - p.x, q.y - p.y) || 1;
      return {
        x: p.x + ((q.x - p.x) / length) * r,
        y: p.y + ((q.y - p.y) / length) * r,
      };
    };
    const before = towards(b, a);
    const after = towards(b, c);
    d += ` L${before.x},${before.y} Q${b.x},${b.y} ${after.x},${after.y}`;
  }
  const last = points.at(-1);
  return `${d} L${last.x},${last.y}`;
}

/**
 * How far the drawn area reaches, for fitting the diagram to the window and
 * sizing an export. Lines count too: a route can loop past the last box.
 */
export function extentOf(nodes, lines = []) {
  let width = 0;
  let height = 0;
  for (const node of Object.values(nodes)) {
    width = Math.max(width, node.x + node.width);
    height = Math.max(height, node.y + node.height);
  }
  for (const line of lines) {
    for (const point of line.points ?? []) {
      // Room for the line's end marks beyond its last point.
      width = Math.max(width, point.x + 16);
      height = Math.max(height, point.y + 16);
    }
  }
  return { width, height };
}

/**
 * Splits off tables with no foreign key in or out. Laid out by elk they become
 * a scatter of one-table "groups" around the real diagram; they read better
 * as a grid of their own beneath it.
 */
export function splitIsolated(graph) {
  const linked = new Set();
  for (const edge of graph.edges) {
    linked.add(edge.sources[0].split(":")[0]);
    linked.add(edge.targets[0].split(":")[0]);
  }
  return {
    connected: {
      ...graph,
      children: graph.children.filter((node) => linked.has(node.id)),
    },
    isolated: graph.children.filter((node) => !linked.has(node.id)),
  };
}

/** Gap above the grid of unrelated tables, where its caption sits. */
export const GROUP_GAP = 72;

/**
 * Places unrelated tables in rows beneath `top`, wrapping at `width` (at least
 * wide enough for four boxes), and returns them with the caption's position.
 */
export function gridBelow(isolated, top, width) {
  const wrap = Math.max(width, 4 * 260);
  const placed = [];
  let x = 0;
  let y = top + GROUP_GAP;
  let rowHeight = 0;
  for (const node of isolated) {
    if (x > 0 && x + node.width > wrap) {
      x = 0;
      y += rowHeight + 40;
      rowHeight = 0;
    }
    placed.push({ ...node, x, y });
    x += node.width + 40;
    rowHeight = Math.max(rowHeight, node.height);
  }
  return {
    children: placed,
    caption: isolated.length
      ? { x: 0, y: top + GROUP_GAP - 24, count: isolated.length }
      : null,
  };
}

/**
 * A stand-in for elkjs, for tests and for the first paint.
 *
 * Drawing the real layout takes a moment on a large schema; boxes in a tidy
 * grid with straight lines are better than a blank panel while it works.
 */
export function plainGraph(graph) {
  const perRow = Math.max(1, Math.ceil(Math.sqrt(graph.children.length)));
  let x = 0;
  let y = 0;
  let rowHeight = 0;
  const children = graph.children.map((node, index) => {
    if (index && index % perRow === 0) {
      x = 0;
      y += rowHeight + 48;
      rowHeight = 0;
    }
    const placed = { ...node, x, y };
    x += node.width + 64;
    rowHeight = Math.max(rowHeight, node.height);
    return placed;
  });
  return positionsFrom({ children, edges: graph.edges });
}

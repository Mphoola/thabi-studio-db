/**
 * Pure helpers for the database explorer tree. Nodes:
 *   { key, type: "database" | "schema" | "group" | "object", label,
 *     database, schema, name?, kind?, count? }
 * Children are loaded lazily and kept per node key by useExplorer.
 */

const GROUPS = [
  { kind: "table", label: "Tables" },
  { kind: "view", label: "Views" },
  { kind: "materialized_view", label: "Materialized Views" },
  { kind: "foreign_table", label: "Foreign Tables" },
];

export function databaseNode(database) {
  return {
    key: `db:${database}`,
    type: "database",
    label: database,
    database,
    schema: null,
  };
}

export function schemaNode(database, schema) {
  return {
    key: `schema:${database}\u0000${schema}`,
    type: "schema",
    label: schema,
    database,
    schema,
  };
}

/**
 * Where a diagram of this node should be drawn, or null if it draws nothing.
 *
 * Both node kinds carry `database` and `schema` and are named by `label` —
 * there is no `name` on either, which is easy to assume and produces a diagram
 * of "undefined". Returns null rather than a half-filled target, so the caller
 * can say so instead of opening an empty tab.
 */
export function diagramTargetFor(node) {
  if (!node || (node.type !== "database" && node.type !== "schema"))
    return null;
  const database = node.database;
  if (!database) return null;
  // A database node has schema: null; on MySQL there is no schema to name.
  const schema = node.type === "schema" ? (node.schema ?? node.label) : null;
  return { database, schema, title: `${node.label ?? database} diagram` };
}

/**
 * A table or view node, keyed exactly as the explorer keys it, so opening it
 * from elsewhere (a foreign key jump) reuses the same tab.
 */
export function objectNode(database, schema, name, kind = "table") {
  const parent = schema ? schemaNode(database, schema) : databaseNode(database);
  return {
    key: `object:${parent.key}\u0000${name}`,
    type: "object",
    label: name,
    name,
    kind,
    database,
    schema: schema || null,
  };
}

/**
 * Groups objects by kind under "Tables", "Views", … in that order. Empty
 * groups are left out. Each group carries its children already.
 */
export function objectGroups(parent, objects) {
  return GROUPS.map(({ kind, label }) => {
    const children = objects
      .filter((object) => object.kind === kind)
      .map((object) => ({
        key: `object:${parent.key}\u0000${object.name}`,
        type: "object",
        label: object.name,
        name: object.name,
        kind: object.kind,
        database: parent.database,
        schema: parent.schema,
      }));
    return {
      key: `group:${parent.key}\u0000${kind}`,
      type: "group",
      label,
      count: children.length,
      database: parent.database,
      schema: parent.schema,
      children,
    };
  }).filter((group) => group.count > 0);
}

/**
 * The rows to render: every node whose ancestors are all expanded, with its
 * depth. `childrenOf(key)` returns loaded children or undefined.
 */
export function visibleRows(roots, expanded, childrenOf) {
  const rows = [];
  const walk = (nodes, level) => {
    for (const node of nodes) {
      rows.push({ node, level });
      if (!expanded.has(node.key)) continue;
      const children = node.children ?? childrenOf(node.key);
      if (children) walk(children, level + 1);
    }
  };
  walk(roots, 1);
  return rows;
}

/** Identifier quoting, matching the Rust side (database/quote.rs). */
export function quoteIdent(engine, name) {
  if (engine === "postgres") return `"${name.replaceAll('"', '""')}"`;
  if (engine === "mysql") return `\`${name.replaceAll("`", "``")}\``;
  return `[${name.replaceAll("]", "]]")}]`;
}

/** Fully qualified, quoted name of an object for the engine. */
export function qualifiedName(engine, node) {
  const parts =
    engine === "postgres"
      ? [node.schema, node.name]
      : [node.database, node.schema, node.name];
  return parts
    .filter(Boolean)
    .map((part) => quoteIdent(engine, part))
    .join(".");
}

/**
 * Builds the exported SVG from the same nodes the panel draws.
 *
 * Pure, so it can be tested without a browser, and deliberately given the
 * nodes rather than the diagram: a file that re-derives its own geometry is a
 * second renderer that can quietly disagree with the screen.
 */

const HEADER = 30;
const ROW = 20;

/**
 * Table columns as drawn, capped and with the rest counted.
 *
 * A caller that passes the wrong thing gets an empty box rather than a thrown
 * error: a drawing helper should not be able to blank the whole panel, and
 * `(tables ?? []).find` on a string throws with a message about `find` that
 * says nothing about which argument was wrong.
 */
export function rowsFor(tables, name, limit = 14) {
  const table = (Array.isArray(tables) ? tables : []).find(
    (one) => one?.name === name,
  );
  if (!table) return { columns: [], hidden: 0 };
  return {
    columns: table.columns.slice(0, limit),
    hidden: Math.max(0, table.columns.length - limit),
  };
}

export function escapeXml(text) {
  const replacements = {
    "<": "&lt;",
    ">": "&gt;",
    "&": "&amp;",
    '"': "&quot;",
    "'": "&apos;",
  };
  return String(text ?? "").replace(/[<>&"']/g, (one) => replacements[one]);
}

/**
 * @param {{ nodes: Record<string, object>, lines: Array<object>, width: number, height: number }} placed
 * @param {Array<object>} tables trimmed tables, as the screen shows them
 */
export function buildSvg(placed, tables) {
  const { nodes, lines, width, height } = placed;
  const parts = [];

  for (const line of lines) {
    parts.push(
      `<line x1="${line.from.x}" y1="${line.from.y}" x2="${line.to.x}" y2="${line.to.y}" ` +
        `stroke="var(--line)" stroke-width="1.5" marker-end="url(#arrow)"/>`,
    );
  }

  for (const node of Object.values(nodes)) {
    const { columns, hidden } = rowsFor(tables, node.id);
    parts.push(
      `<rect x="${node.x}" y="${node.y}" width="${node.width}" height="${node.height}" ` +
        `rx="6" fill="var(--surface)" stroke="var(--line)"/>`,
      `<rect x="${node.x}" y="${node.y}" width="${node.width}" height="${HEADER}" ` +
        `rx="6" fill="var(--surface-raised)"/>`,
      `<line x1="${node.x}" y1="${node.y + HEADER}" x2="${node.x + node.width}" ` +
        `y2="${node.y + HEADER}" stroke="var(--line)"/>`,
      `<text x="${node.x + 10}" y="${node.y + 19}" font-size="12" font-weight="600" ` +
        `fill="var(--fg)">${escapeXml(node.id)}</text>`,
    );

    columns.forEach((column, index) => {
      const y = node.y + HEADER + index * ROW + 13;
      parts.push(
        `<text x="${node.x + 10}" y="${y}" font-size="11" ` +
          `font-weight="${column.primaryKey ? 700 : 400}" ` +
          `fill="${column.primaryKey ? "var(--fg)" : "var(--fg-muted)"}">` +
          `${escapeXml(column.name)}</text>`,
        `<text x="${node.x + node.width - 10}" y="${y}" font-size="10" text-anchor="end" ` +
          `fill="var(--fg-subtle)">${escapeXml(column.dataType)}</text>`,
      );
    });

    if (hidden)
      parts.push(
        `<text x="${node.x + 10}" y="${node.y + node.height - 4}" font-size="10" ` +
          `fill="var(--fg-subtle)">+${hidden} more</text>`,
      );
  }

  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" ` +
    `viewBox="0 0 ${width} ${height}">` +
    '<defs><marker id="arrow" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" ' +
    'markerHeight="7" orient="auto"><path d="M0,0 L8,4 L0,8 z" fill="var(--line)"/>' +
    "</marker></defs>" +
    `<rect width="${width}" height="${height}" fill="var(--bg)"/>` +
    parts.join("") +
    "</svg>"
  );
}

/** A file name from a tab title, with anything a filesystem would object to removed. */
export function fileNameFor(title) {
  return `${String(title ?? "diagram").replace(/[^\w.-]+/g, "-")}.svg`;
}

/**
 * Hands an SVG to the user as a file.
 *
 * The object URL is revoked immediately: on some platforms a download that has
 * not started yet can be cancelled by a revoked URL, and the leak is otherwise
 * a blob per export for the life of the window.
 */
export function downloadSvg(svg, filename) {
  const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

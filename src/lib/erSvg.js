/**
 * Builds the exported SVG from the same nodes the panel draws.
 *
 * Pure, so it can be tested without a browser, and deliberately given the
 * nodes rather than the diagram: a file that re-derives its own geometry is a
 * second renderer that can quietly disagree with the screen.
 */

import {
  HEADER_HEIGHT as HEADER,
  ROW_HEIGHT as ROW,
  roundedPath,
} from "./erLayout.js";

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

/** The colour tokens the drawing uses, read from the page for an export. */
export const SVG_TOKENS = [
  "canvas",
  "surface",
  "surface-raised",
  "line",
  "line-strong",
  "fg",
  "fg-muted",
  "fg-subtle",
  "accent-subtle",
  "accent-text",
  "warning-subtle",
  "warning-text",
];

/**
 * @param {{ nodes: Record<string, object>, lines: Array<object>, width: number, height: number }} placed
 * @param {Array<object>} tables trimmed tables, as the screen shows them
 * @param {Record<string, string>} colors token -> colour. A standalone file
 *   and the canvas a PNG is drawn on can't see the app's CSS variables, so
 *   an export passes the resolved colours; without them `var(--…)` is kept.
 * @param {Set<string>} keyed "table:column" of every foreign key column
 */
export function buildSvg(placed, tables, colors = {}, keyed = new Set()) {
  const { nodes, lines, width, height } = placed;
  const c = (token) => colors[token] || `var(--${token})`;
  const font =
    'font-family="-apple-system, BlinkMacSystemFont, Segoe UI, sans-serif"';
  const mono = 'font-family="ui-monospace, SFMono-Regular, Menlo, monospace"';
  const parts = [];

  for (const line of lines) {
    parts.push(
      `<path d="${roundedPath(line.points ?? [line.from, line.to])}" fill="none" ` +
        `stroke="${c("fg-subtle")}" stroke-width="1.4" ` +
        'marker-start="url(#many)" marker-end="url(#one)"/>',
    );
  }

  for (const node of Object.values(nodes)) {
    const { columns, hidden } = rowsFor(tables, node.id);
    const { x, y, width: w } = node;
    parts.push(
      `<rect x="${x}" y="${y}" width="${w}" height="${node.height}" rx="8" ` +
        `fill="${c("surface")}" stroke="${c("line-strong")}"/>`,
      `<path d="M${x + 0.5},${y + HEADER} V${y + 8} a7.5,7.5 0 0 1 7.5,-7.5 ` +
        `H${x + w - 8} a7.5,7.5 0 0 1 7.5,7.5 V${y + HEADER} Z" fill="${c("surface-raised")}"/>`,
      `<line x1="${x}" y1="${y + HEADER}" x2="${x + w}" y2="${y + HEADER}" stroke="${c("line")}"/>`,
      `<text x="${x + 12}" y="${y + 21.5}" ${font} font-size="12.5" font-weight="600" ` +
        `fill="${c("fg")}">${escapeXml(node.id)}</text>`,
    );

    columns.forEach((column, index) => {
      const top = y + HEADER + index * ROW;
      const foreign = keyed.has(`${node.id}:${column.name}`);
      if (column.primaryKey || foreign) {
        const kind = column.primaryKey ? "warning" : "accent";
        parts.push(
          `<rect x="${x + 10}" y="${top + 5}" width="20" height="12" rx="3" fill="${c(`${kind}-subtle`)}"/>`,
          `<text x="${x + 20}" y="${top + 14}" ${font} font-size="8" font-weight="700" ` +
            `text-anchor="middle" fill="${c(`${kind}-text`)}">${column.primaryKey ? "PK" : "FK"}</text>`,
        );
      }
      parts.push(
        `<text x="${x + 38}" y="${top + 15}" ${font} font-size="12" ` +
          `font-weight="${column.primaryKey ? 700 : 400}" fill="${c("fg")}">` +
          `${escapeXml(column.name)}</text>`,
        `<text x="${x + w - 12}" y="${top + 15}" ${mono} font-size="11" text-anchor="end" ` +
          `fill="${c("fg-muted")}">${escapeXml(column.dataType)}</text>`,
      );
    });

    if (hidden)
      parts.push(
        `<text x="${x + 38}" y="${y + HEADER + columns.length * ROW + 15}" ${font} ` +
          `font-size="11" fill="${c("fg-subtle")}">${hidden} more columns</text>`,
      );
  }

  const marks =
    '<marker id="many" viewBox="-2 -8 18 16" refX="0" refY="0" markerWidth="18" ' +
    'markerHeight="16" markerUnits="userSpaceOnUse" orient="auto">' +
    `<path d="M0,-6 L11,0 L0,6 M0,0 L11,0 M14,-6 L14,6" fill="none" stroke="${c("fg-subtle")}" stroke-width="1.4"/></marker>` +
    '<marker id="one" viewBox="-16 -8 18 16" refX="0" refY="0" markerWidth="18" ' +
    'markerHeight="16" markerUnits="userSpaceOnUse" orient="auto">' +
    `<path d="M-6,-6 L-6,6 M-11,-6 L-11,6" fill="none" stroke="${c("fg-subtle")}" stroke-width="1.4"/></marker>`;

  return (
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" ` +
    `viewBox="0 0 ${width} ${height}">` +
    `<defs>${marks}</defs>` +
    `<rect width="${width}" height="${height}" fill="${c("canvas")}"/>` +
    parts.join("") +
    "</svg>"
  );
}

/** Reads the colour tokens from the page, for an export that can't see them. */
export function resolvedColors(element = document.documentElement) {
  const style = getComputedStyle(element);
  return Object.fromEntries(
    SVG_TOKENS.map((token) => [
      token,
      style.getPropertyValue(`--${token}`).trim(),
    ]),
  );
}

/** A file name from a tab title, with anything a filesystem would object to removed. */
export function fileNameFor(title, extension = "svg") {
  return `${String(title ?? "diagram").replace(/[^\w.-]+/g, "-")}.${extension}`;
}

/**
 * A PNG of the SVG at `scale` times its size, as base64, by drawing it on a
 * canvas. The SVG paints its own background, so the PNG looks like the
 * screen whatever the viewer's default is.
 */
export async function svgToPng(svg, width, height, scale = 2) {
  const image = new Image();
  image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
  await image.decode();
  const canvas = document.createElement("canvas");
  canvas.width = Math.ceil(width * scale);
  canvas.height = Math.ceil(height * scale);
  const context = canvas.getContext("2d");
  context.scale(scale, scale);
  context.drawImage(image, 0, 0, width, height);
  return canvas.toDataURL("image/png").split(",")[1];
}

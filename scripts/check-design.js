// Design guardrails, run by `npm run check:design`:
//  1. Key colour pairs in src/styles/tokens.css meet WCAG contrast targets
//     in both light and dark themes.
//  2. Components and scripts contain no hard-coded colours.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

// fileURLToPath, not URL.pathname, so Windows paths work too.
const root = fileURLToPath(new URL("..", import.meta.url));
const tokensCss = readFileSync(join(root, "src/styles/tokens.css"), "utf8");

function readBlock(selector) {
  const start = tokensCss.indexOf(`${selector} {`);
  const end = tokensCss.indexOf("\n}", start);
  const vars = {};
  for (const [, name, value] of tokensCss
    .slice(start, end)
    .matchAll(/--([\w-]+):\s*(#[0-9a-f]{6})\s*;/gi)) {
    vars[name] = value;
  }
  return vars;
}

function luminance(hex) {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a, b) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

// [foreground, background, minimum ratio]
const TEXT = 4.5;
const LARGE_OR_UI = 3;
const pairs = [
  ...[
    "canvas",
    "sidebar",
    "toolbar",
    "surface",
    "surface-raised",
    "surface-hover",
    "surface-active",
  ].flatMap((bg) => [
    ["fg", bg, TEXT],
    ["fg-muted", bg, TEXT],
  ]),
  ["fg-subtle", "canvas", LARGE_OR_UI],
  ["fg-subtle", "sidebar", LARGE_OR_UI],
  ["fg-subtle", "surface", LARGE_OR_UI],
  ["accent-fg", "accent", TEXT],
  ["accent-fg", "accent-hover", TEXT],
  ["accent-text", "canvas", TEXT],
  ["accent-text", "surface", TEXT],
  ["accent-text", "accent-subtle", TEXT],
  ["fg", "accent-subtle", TEXT],
  ["fg", "accent-muted", TEXT],
  ["fg", "selection", TEXT],
  ["danger-fg", "danger", TEXT],
  ["danger-fg", "danger-hover", TEXT],
  ["danger-text", "canvas", TEXT],
  ["danger-text", "danger-subtle", TEXT],
  ["success-fg", "success", LARGE_OR_UI],
  ["success-text", "canvas", TEXT],
  ["success-text", "success-subtle", TEXT],
  ["warning-fg", "warning", LARGE_OR_UI],
  ["warning-text", "canvas", TEXT],
  ["warning-text", "warning-subtle", TEXT],
  ["line-strong", "surface", 1.4],
  ["grid-null", "canvas", LARGE_OR_UI],
  ...["keyword", "string", "number", "function", "type", "operator"].map(
    (s) => [`syntax-${s}`, "canvas", TEXT],
  ),
  ["syntax-comment", "canvas", LARGE_OR_UI],
  ...["postgres", "mysql", "sqlserver"].map((e) => [
    `engine-${e}`,
    "sidebar",
    LARGE_OR_UI,
  ]),
  ...["red", "orange", "yellow", "green", "teal", "blue", "purple", "gray"].map(
    (t) => [`tag-${t}`, "sidebar", LARGE_OR_UI],
  ),
];

const failures = [];
const light = readBlock(":root");
const dark = { ...light, ...readBlock('[data-theme="dark"]') };

for (const [theme, vars] of [
  ["light", light],
  ["dark", dark],
]) {
  for (const [fg, bg, min] of pairs) {
    if (!vars[fg] || !vars[bg]) {
      failures.push(`${theme}: missing token --${!vars[fg] ? fg : bg}`);
      continue;
    }
    const ratio = contrast(vars[fg], vars[bg]);
    if (ratio < min) {
      failures.push(
        `${theme}: --${fg} on --${bg} is ${ratio.toFixed(2)}:1 (needs ${min}:1)`,
      );
    }
  }
}

// No hard-coded colours outside the token file.
const COLOUR =
  /#[0-9a-f]{3,8}\b|\b(?:rgba?|hsla?|oklch|oklab|lab|lch)\(|-(?:white|black|(?:slate|gray|zinc|neutral|stone|red|orange|amber|yellow|lime|green|emerald|teal|cyan|sky|blue|indigo|violet|purple|fuchsia|pink|rose)-\d{2,3})\b/i;
function walk(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}
for (const file of walk(join(root, "src"))) {
  if (!/\.(vue|js|css)$/.test(file) || file.endsWith("tokens.css")) continue;
  readFileSync(file, "utf8")
    .split("\n")
    .forEach((line, i) => {
      if (COLOUR.test(line)) {
        failures.push(
          `${relative(root, file)}:${i + 1} hard-coded colour: ${line.trim()}`,
        );
      }
    });
}

if (failures.length) {
  console.error(`Design check failed:\n  ${failures.join("\n  ")}`);
  process.exit(1);
}
console.log(
  `Design check passed (${pairs.length * 2} contrast pairs, no hard-coded colours).`,
);

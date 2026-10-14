# 0002 — Colour system

**Status:** accepted · **Date:** 2026-10-14

## Decision

- Cool graphite neutrals with an iris (blue-violet) accent, in light and dark.
  Dark mode is a separate palette, not an inversion: surfaces get lighter as
  they come forward, and accent text is lifted for contrast.
- All colours are semantic tokens in `src/styles/tokens.css`: surfaces,
  lines, text, accent, status (each with fill, `-fg`, `-text` and `-subtle`),
  focus, selection, results grid, SQL syntax, database engines and
  connection tags.
- Tailwind's built-in palette is disabled (`--color-*: initial`), so only
  tokens produce colour utilities.
- `npm run check:design` (part of `npm run check`) verifies 55 contrast pairs
  per theme against WCAG AA (4.5:1 text, 3:1 UI and large text) and fails on
  any hex, `rgb()`/`hsl()`/`oklch()` value or palette class outside the token file.
- Environment tags follow TablePlus: Local, Development, Testing, Staging,
  Production, mapped to tag colours. See `docs/design/workspace.md`.

## Why

The app should feel premium and calm, and colour should only ever carry
meaning. Enforcing tokens in CI keeps it that way as more screens are added.

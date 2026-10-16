# Thabi Studio brand

**Name.** Thabi is short for Tabitha Mphoola, the developer's daughter.
Tabitha means "gazelle": quick, light and sure-footed.

**Mark** (`logo.svg`). A "T" built from a database: the crossbar is a disc
(a cylinder seen from above) and the stem a column of stacked records. The
small spark is Tabitha's.

**Colours.** The iris accent of the app (`--accent` `#5651D6` in
`src/styles/tokens.css`), on a gradient from `#7B73FF` to `#2B2784`, with
white and lavender (`#EEECFF`) for the mark.

**Files**

- `logo.svg`: the source. Edit this, never the PNGs.
- `logo.png`: 1024 × 1024 render, the input for the app icons.
- App icons in `src-tauri/icons/` are generated from it:

  ```bash
  npx tauri icon branding/logo.png
  ```

  (Delete the `android/` and `ios/` folders it also writes; the app is
  desktop only.)

- `src/assets/logo.svg`: a copy used in the About window.

To re-render `logo.png` after editing the SVG, open it in a Chromium
browser headlessly at 1024 × 1024 with a transparent background (ImageMagick
does not draw the gradients correctly).

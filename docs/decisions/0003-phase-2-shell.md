# 0003 — Phase 2: workspace shell and component set

**Status:** accepted · **Date:** 2026-10-14

Implements the layout in `docs/design/workspace.md`.

## Decisions

1. **Icons: `lucide-vue-next`** (ISC licence, tree-shaken per icon). One
   family, 14px in controls and 16px in toolbars, stroke width 1.75.
2. **Unified macOS title bar.** `titleBarStyle: "Overlay"`, `hiddenTitle`,
   traffic lights inset to suit the 38px bar. Dragging uses
   `data-tauri-drag-region`, which needs the
   `core:window:allow-start-dragging` permission (the only one added).
3. **Commands and shortcuts.** Every action is a registered command
   (`useCommands`) with an optional shortcut string like `Mod+Shift+K`.
   One global listener dispatches shortcuts; the command palette (⌘P) and
   the Keyboard Shortcuts dialog (⌘/) read the same registry, so they can't
   drift. Shortcuts match on the physical key so Shift and Option layouts work.
4. **Floating UI in the top layer.** Menus use `popover="auto"` (light
   dismiss, Escape), tooltips and toasts use `popover="manual"`, so all of
   them sit above modal dialogs. Positioning is a small helper
   (`lib/position.js`) rather than a library. This resolves the toast
   limitation noted in 0001.
5. **Interim UI preferences in `localStorage`:** theme, sidebar width and
   whether the sidebar is open. These are non-secret and move to the
   SQLite `app_settings` table in Phase 3.
6. **Narrow windows:** below 1000px the sidebar starts hidden and ⌘1 shows
   it; the window minimum stays 860 × 560.
7. **Vitest** added for pure frontend logic (shortcut parsing/matching,
   fuzzy filtering). Component tests can come later if needed.
8. **Component gallery** (`src/components/dev/`) is loaded only when
   `import.meta.env.DEV` is true and is absent from release bundles.
   Open it from ⌘P → "Show Component Gallery". (Removed on 2026-10-16;
   see `0034`.)

## Not done in Phase 2, by design

- No connection, explorer, tab or editor behaviour: those are Phases 3–7.
  The New Connection button is visibly disabled with a note, and the
  connection pill only says "No connection".

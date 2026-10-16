# 0026 — Row detail panel

**Status:** accepted · **Date:** 2026-10-15 · Phase 12, `plan.md` §7b

A grid is for comparing rows at a glance. Sometimes you want to read one
record properly — a long description, a JSON document, every column with its
type. That is the row detail panel, on Mod+Alt+I and in the command palette.

## Decisions

1. **A dialog, not a side panel, for now.** The plan describes TablePlus's
   right-hand panel. A dialog is what this app already uses for the value
   inspector and it needs no reflow of the workspace layout; when multiple
   windows land in Phase 11 a docked panel has somewhere to live.
2. **Every column, with its type, and the value in full.** The grid clips
   with an ellipsis, which is right for scanning and wrong for reading. Long
   text and JSON wrap out here instead, NULL is still NULL, and binary still
   says it is truncated — the panel shows what the result holds, not more.
3. **Editing goes through the same staged changes as the grid.** The panel
   edits a value by calling the same `useRowEdits` API the grid cells use, so
   the two can never disagree about what is pending, and Apply behaves
   identically from either. A table that is read-only here is read-only
   because the connection says so, not because the panel forgot to check.
4. **Opened from the keyboard, the context menu or the palette.** The grid
   registers a small opener (`useRowDetail`) so the command palette can reach
   it without a ref threaded through the tree, following the pattern the other
   dialogs already use.

## A bug the app run caught

The first version passed every unit test and lint, and broke the **command
palette and the shortcuts dialog** in the running app: the registry calls
`command.enabled()` as a predicate and I had passed a ref object, so rendering
any list of commands threw. Unit tests could not have caught it — the registry
is only exercised through the UI. It is fixed, and the shortcut is the kind
thing a clean launch should be asked about every now and then.

## Verified

- `npm run check` green, and a clean launch of the desktop app with no
  console errors in the command palette or the shortcuts dialog.

# 0025 — Choosing and ordering columns

**Status:** accepted · **Date:** 2026-10-15 · Phase 12, `plan.md` §7b

A table with forty columns is unusable when thirty of them are wide blobs. The
grid now shows the columns the user wants, in the order they want.

## Decisions

1. **The layout is stored as source column indexes**, held on the tab, so a
   table remembers its layout next time and it survives switching tabs. It is
   not a user preference, because it is about this table, not this app.
2. **A stale layout is healed, not trusted.** A stored order that names a
   column the table no longer has is dropped, and a column missing from the
   order is appended in its own order. A renamed or dropped column can
   therefore never leave a grid with a column missing that the user never hid.
3. **The grid speaks display positions; the source index is mapped explicitly.**
   Every component inside the grid addresses a display position and calls
   `visible[position]` to reach the real column, so selection, editing, the
   cell picker and copy keep addressing the right column after a reorder. This
   is the part worth being careful about: it is where a reordering bug would
   write to the wrong field.
4. **Reorder by drag, and by arrow buttons.** Dragging a row in the Columns
   dialog reorders it; the up and down buttons do the same and work from the
   keyboard, because a mouse-only reorder would be the one grid action a
   keyboard user could not do.
5. **Hidden columns keep their place.** Moving a visible column swaps it with
   its neighbour in the visible order and leaves hidden columns in their own
   slots, so showing one again puts it where it was.
6. **"Fit to contents" measures the rows already loaded** and is labelled as
   such by being what it is: a width that fits what has been read. Reading the
   whole table to size a column is not something to do behind a click.
7. **A manual resize wins over fit** until the columns change.

## Verified

- Unit (`tests/frontend/columns.test.js`): the default layout, a stored order
  with hidden columns, healing a layout that names a column that no longer
  exists, dropping a stale hidden entry, hide/show by name, moving a column
  both ways and back, refusing to move past either end, keeping a hidden
  column in place across a move, fit widths staying in range, and reset.

## Not done

Column visibility is per tab, not per user, and is not yet shared between a
table opened in the explorer and the same table in a future "open table in new
tab" flow.

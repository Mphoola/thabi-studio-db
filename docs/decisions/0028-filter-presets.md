# 0028 — Saved filter presets per table

**Status:** accepted · **Date:** 2026-10-15 · Phase 12, `plan.md` §7b

The filter bar can express a lookup, and the lookups a table is usually read
with get built again every time. A preset is the filters as they were saved, so
applying one is putting them back rather than translating them.

## Decisions

1. **A preset belongs to a table on a connection, not to a tab.** It is in the
   app's SQLite (migration 0013), so it survives closing the tab and restarting
   the app. A tab-scoped copy would have been simpler and would have gone the
   moment the tab did — which is exactly when a lookup you keep retyping is most
   annoying.
2. **Scoped to one object.** Keyed by connection, database, schema and object.
   A filter named "Recent orders" means nothing on another server or another
   table, and offering it there would be a wrong answer rather than a rough one.
   The connection is `ON DELETE CASCADE`: deleting a connection takes its
   presets with it rather than leaving them behind for a server that is gone.
3. **Filters are stored as the bar wrote them**, as JSON, and validated on the
   way in: a non-empty list of at most 50 objects. An empty preset would be a
   name in the list that quietly shows every row when chosen.
4. **Saving over a name updates that preset.** Names are compared
   case-insensitively within a table, so "Recent orders" and "recent orders" are
   one preset, and the name typed last is the one kept. Two presets that differ
   only in case would be indistinguishable in the menu.
5. **A row that will not parse is skipped, not fatal.** One corrupt preset does
   not take the rest of the list with it.
6. **Applying is the webview's job.** `apply` hands the filters back to the bar
   exactly as stored, including the raw-SQL marker, so a preset round-trips
   through the same code path the bar uses rather than a second one.

7. **A migration that has run cannot be changed by editing its file.** The
   first version of 0013 created the timestamps as `NOT NULL` with no default,
   so every INSERT failed — "Local data couldn't be accessed", with the SQLite
   message discarded by the storage mapping. Correcting the file fixed new
   databases and the tests, and did nothing for a database that had already
   applied 0013, which is where the bug was actually reported from. 0014
   rebuilds the table for those, and a test asserts the default exists rather
   than inferring it from an insert succeeding.

## Known limits

- Presets are not shared between connections, and there is no export; a lookup
  someone else set up has to be typed once here.
- A preset whose column has since been dropped applies as it is, and the server
  is the one to say the column is not there — the same as a hand-built filter.

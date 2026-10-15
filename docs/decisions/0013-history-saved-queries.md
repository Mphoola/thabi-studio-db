# 0013 — Phase 9: history, saved queries, recovery and polish

**Status:** accepted · **Date:** 2026-10-15

## Decisions

1. **Query history is recorded in Rust** after every statement a query tab
   runs: connection name, database, SQL (up to 100,000 characters), time,
   duration, status (completed, failed, cancelled, timed out), row count
   and a sanitised error summary. It never fails the query. History can be
   turned off; entries older than the retention setting (default 30 days)
   and beyond 10,000 entries are pruned on each insert. Search uses SQL
   `LIKE` with escaped wildcards.
2. **History panel:** newest first, with status icon, SQL, connection,
   database, duration, rows and "5 min ago". Click opens the SQL in a new tab
   with the same connection and database; right-click copies or deletes;
   **Clear History…** (also in Settings) asks first.
3. **Saved queries:** name, SQL, optional connection and notes. **Save
   Query** (⌘S / Ctrl+S) saves the active tab; the first save asks for a
   name. A saved tab shows its name and the unsaved dot appears only when
   its SQL changes. Opening a saved query that's already open switches to
   its tab. Rename and delete from the Saved panel.
4. **Open-tab recovery:** open query tabs (title, SQL, connection,
   database, saved query) are written to SQLite 0.8 s after any change and
   restored on launch, so unsaved SQL survives quitting or a crash. Results
   aren't restored (they may be large and stale).
5. **Preferences:** appearance, editor font size (11–18 pt), query
   timeout, rows kept per result, history on/off, history retention.
6. **Shortcuts added:** ⌘S Save Query, ⌘E Focus Editor (Ctrl on Windows
   and Linux). All are in the Query menu, Open Anything and the Keyboard
   Shortcuts list.
7. **Privacy note** in About (what's stored where, the local data folder)
   and `docs/PRIVACY.md`.
8. **Accessibility pass:** controls without visible labels (search boxes,
   selects in Settings, icon buttons) have accessible names; lists, the
   explorer tree, the results grid and tabs use ARIA roles with keyboard
   navigation; dialogs are native `<dialog>` elements with focus trapping
   and Escape; destructive confirmations focus Cancel.

## Not done

- Restored tabs whose saved query was edited show as saved until changed
  again (the saved text isn't stored with the tab).

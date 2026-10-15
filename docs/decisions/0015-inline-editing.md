# 0015 — Inline editing of table data

Status: accepted (2026-10-15). Added as its own milestone after
connections and queries were proven, as `plan.md` §2 requires.

## What the user can do

On a table's Data view (not views), when the table has a primary key whose
columns are all in the result:

- Double-click a cell, or press Return or F2, to edit it. Return saves and
  moves down, Tab moves right, Escape cancels, Option/Alt+Return adds a
  line break. Right-click offers Edit Cell, Set to NULL, Delete Rows and
  Revert Changes.
- Delete or Backspace marks the selected rows for deletion (press again to
  unmark).
- Add Row (+ in the header) stages a new row; blank fields use the column
  default.

Nothing is sent until Apply (or Mod+S). Edited cells are highlighted,
deleted rows are struck through, and a bar shows the counts with Discard,
Review SQL… and Apply. The tab shows the unsaved dot and asks before
closing. Binary cells can't be edited.

## How it is applied

- Rows are identified by their primary key, so staged edits survive
  sorting, paging and reloading.
- Rust builds the statements (`database/edit.rs`): deletes, then updates,
  then inserts. Identifiers are quoted per engine; values are the grid's
  text written as literals (`quote::literal`) that the server converts to
  the column type, as for row filters (0014). The user can review every
  statement before it runs.
- `QueryTab::apply` runs them on the table tab's own session inside one
  transaction. Every statement must change exactly one row (SQL Server
  uses `@@ROWCOUNT`; MySQL reports matched rows). Otherwise, or on any
  error, it rolls back and nothing is saved. If the rollback fails, the
  session is dropped so the server discards the transaction. Nothing is
  retried. A connection lost during apply is reported as an unknown
  outcome.
- The app asks before applying any change on a Production connection, and
  before applying deletes anywhere.

## Limits

- Tables without a primary key are read-only.
- New rows show in the changes bar and review, not as grid rows.
- Optimistic concurrency is by key only: an edit overwrites a value someone
  else changed since the rows were read, but a deleted or re-keyed row is
  caught (it changes 0 rows).

## Verified

Unit tests for SQL generation and the frontend edit state; integration
tests (`*_row_edits` in `tests/queries.rs`) apply an update, delete and
insert, then prove a failing change rolls back the rest, on MySQL 8.4,
PostgreSQL 16 and SQL Server 2022.

# 0014 — Connection rail and table row filters

Status: accepted (2026-10-15). Ideas taken from TablePlus.

## Connection rail

Open connections (connected or connecting) are listed in a 64px rail left
of the sidebar, each with its engine colour and environment dot. Clicking
one selects it (the explorer, title bar and status bar follow); right-click
offers Edit Connection… and Disconnect. The rail is hidden when nothing is
open, and its + button opens Switch Connection (Mod+Shift+K). It reuses the
existing per-connection state; nothing new is stored.

## Row filters on table data

The Filter Rows button on a table's Data view shows a filter bar. Each row
is column + operator + value, or Raw SQL (one condition, no semicolons).
Rows are AND-ed and applied with Return or Apply; Clear removes them.
Filters are kept on the tab.

`database/filter.rs` builds the WHERE clause in Rust. Column names go
through `quote::ident`. Values are written as escaped string literals
(PostgreSQL `''`, MySQL `''` and `\\`, SQL Server `N''`) rather than bound,
because object-tab sessions run text and the filter is the user's own
condition on their own session, equivalent to SQL typed in the editor. The
servers coerce the literal to the column type (`id = '71'` works on an
integer column). LIKE operators cast the column to text so they work on any
type; `%` and `_` in a value act as wildcards, as in TablePlus.

Integration tests (`filters` in `tests/drivers.rs`) cover all three engines.

## Not yet

Inline editing of rows (staged UPDATE/INSERT/DELETE run in one
transaction) is planned as its own phase; see plan.md §2.

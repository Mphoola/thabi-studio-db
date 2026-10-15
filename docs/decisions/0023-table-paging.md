# 0023 — Reading a table a page at a time

**Status:** accepted · **Date:** 2026-10-15 · Phase 12, `plan.md` §7b

Until now a table tab fetched up to the row limit once and paged through
those rows in memory. A million-row table was therefore as slow to open as a
million rows of work, and sorting only reordered what had been fetched.
Pages now come from the server.

## Decisions

1. **`LIMIT`/`OFFSET`, ordered by the primary key.** `database::paging` builds
   the statement per engine: `LIMIT … OFFSET …`, and `OFFSET … ROWS FETCH NEXT
… ROWS ONLY` on SQL Server, which cannot page without an `ORDER BY` — a
   view with no key gets `ORDER BY (SELECT NULL)` so paging at least works.
2. **A stable order across pages.** Rows come back ordered by the primary
   key, and when the user sorts by a column the key is appended as a
   tie-breaker. Without it, rows with equal sort values could land on two
   pages or on none. Sorting on a key column doesn't repeat it.
3. **Sorting re-queries.** `fetch_table_page` takes the sort, so a table tab
   sorts the whole table rather than the page it happens to be showing.
4. **The sort column is quoted, never interpolated.** It arrives from the grid
   as a name and goes through `quote::ident` like every other identifier. A
   column called `id"; DROP TABLE orders; --` is one identifier, and a test
   says so.
5. **Counts are labelled honestly.** The first read returns the catalog's
   estimate (`reltuples`, `information_schema.TABLE_ROWS`,
   `sys.dm_db_partition_stats`) and no exact number; `count_table_rows` gives
   the exact count when asked, since it costs a scan. A failure to estimate is
   swallowed and reported as unknown — a view, or statistics that were never
   gathered, must not stop the rows from loading.
6. **The grid didn't change.** `useGridData` already asked a source for
   `load(offset, limit, sort)`; `tableSource` is a new source behind the same
   interface, so paging, caching and virtual scrolling are untouched.
7. **A page is never larger than a page.** A query tab hands back
   `FIRST_PAGE` rows and keeps the rest, so `fetch_table_page` clamps to that.
   Asking for more would return fewer rows than the grid counted on, and the
   page arithmetic would be quietly wrong.

## Known gap, on purpose

Copy and CSV export for a table tab are **absent** rather than half-working:
both need the whole table re-read from the server, which is the next
increment. A source without them simply doesn't offer those menu items, rather
than exporting page 1 and calling it the table. Query results are unaffected.

## Verified

- Unit: paging SQL for all three engines, the tie-breaker, the clamped sort
  column, filters carried through, the keyless view, and quoted estimates and
  counts.
- Integration: `*_pages_a_large_table` inserts 10,000 rows across two tenants
  (so a page boundary lands inside a key run), walks the table a page at a
  time, and asserts every row comes back exactly once in key order; then that
  sorting on the server returns the largest value first, and that the exact
  count agrees. Passes on PostgreSQL 16, MySQL 8.4 and SQL Server 2022.

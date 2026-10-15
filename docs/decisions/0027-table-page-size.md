# 0027 — How many rows a table shows at once, and moving between them

**Status:** accepted · **Date:** 2026-10-15 · Phase 12, `plan.md` §7b

0023 made table pages come from the server, but the page was a fixed 200 rows
and the grid only moved through them by scrolling. A table with 1.1 million rows
opened as 200 rows of nothing and no way to say "page 500".

## Decisions

1. **A page is the connection's row limit.** The connection profile already has
   one (`maxResultRows`), and it already answers "how many rows at once" for a
   query result. Reusing it means the user does not set the same number twice.
   A connection that sets none gets 1,000 — enough to be useful, few enough that
   a million-row table opens at once.
2. **A page is at most 100,000 rows.** A connection may ask for more rows than
   are worth holding together, and paging stops helping at that size. The server
   clamps to the same number (`paging::MAX_TABLE_PAGE`), so the ceiling is
   enforced on both sides rather than trusted from the webview.
3. **The grid scrolls a window, not the whole table.** The scroller is as tall as
   what is _loaded_, and grows a page at a time as the user scrolls. Sizing it to
   the table instead would mean a 1.1-million-row spacer — 26 million pixels,
   past what a browser will lay out — and would hold every row scrolled past in
   the webview, which is the thing paging exists to avoid.
4. **A jump moves the window.** "Last page" on a million-row table loads that
   page and nothing else. The alternative — loading the 500,000 rows before it —
   is the behaviour this replaced.
5. **The end of the table is a short page, not an estimate.** Catalog statistics
   lie. "Next" follows what the server actually returned, so a stale estimate
   cannot leave a user clicking forever at a page that is not there.
6. **A page count needs a real total.** The unknown-size stand-in is
   `Number.MAX_SAFE_INTEGER`, which is finite — so anything reading the total
   asks `knownTotal` rather than `Number.isFinite`, or the pager labels a table
   with no known size as having nine quadrillion rows. (This was a bug, caught by
   the tests the increment added.)
7. **The page size is read from one place.** `useGridData` asks the source for
   it; a query result has none and keeps the 200 rows it always sent. Every
   position-to-offset conversion goes through it, since a page half the size it
   was asked to be would show the wrong rows quietly.

8. **A table's results bar says nothing about rows.** A query result's `total`
   is how many rows were fetched and being cut short is worth warning about; a
   table's `total` is how many rows the table has and being cut short is the
   whole point, since the pager is how the rest is reached. Reading one with the
   other's rules put "First 1,109,100 rows fetched — add a WHERE or LIMIT" above
   "Rows 1–1,000 of about 1,109,100". The source carries a `kind`, and a table's
   bar keeps to the elapsed time, which is all it can stand behind.
9. **One request at a time.** A tab runs one statement, and the server answers a
   second with "a query is already running on this tab" — which a scroll and a
   page jump can provoke together. Page reads queue in the grid rather than
   racing, and a jump waits for its rows before it scrolls.

## Known limits

- The pager says "of about N" for a catalog estimate and only "of N" after an
  exact count, which the user has to ask for.
- Sorting re-queries from page 0 and drops the window.
- A jump is one request; a sort is another. Both are visible as the usual row
  read.

# 0010 — Results grid, inspection and CSV export

**Status:** accepted · **Date:** 2026-10-15

## Decisions

1. **Results live in Rust, not the webview.** A statement's rows are
   fetched up to the "Rows kept per result" setting (default 10,000, up to
   1,000,000) and kept by the query tab (`src-tauri/src/results.rs`). The UI
   gets the first 200 rows and asks for 200-row pages as the grid scrolls.
   Each tab keeps its last 8 results; an older one says "Run the query
   again". When the limit is reached the footer says so and suggests a
   WHERE/LIMIT or a higher setting. (Re-running with OFFSET was rejected:
   it changes what the statement does and isn't safe for writes.)
2. **Virtualised grid** (`src/components/grid/`): only visible rows (plus a
   small overscan) are in the DOM. Sticky header and row numbers, resizable
   columns, click-to-sort (ascending → descending → off). Sorting happens in
   Rust over all kept rows: numeric types by value, text case-insensitively,
   NULLs always last. It sorts fetched rows only, which the UI makes clear
   by never re-querying.
3. **Values.** NULL is italic "NULL"; an empty string shows as `''`;
   binary shows its size (‹2,048 bytes›) with a hex preview; numbers are
   right-aligned and never pass through a JavaScript number; long or
   multi-line text stays on one line. **Inspect Value** (Return, double-click
   or the context menu) shows the full value, pretty-prints JSON, and copies it.
4. **Selection and copy.** Click, Shift-click and arrow/Shift-arrow make a
   rectangular selection; ⌘A selects all. ⌘C copies TSV, ⇧⌘C with headers;
   the context menu adds Copy as CSV and Copy Rows. Copies are built in Rust
   from the kept rows, so they match what's shown (current sort, selected
   columns only) and include full binary data as hex. The native clipboard
   plugin is used because the webview refuses writes after an await.
5. **CSV export.** Export CSV… opens the native save dialog; Rust streams
   the rows to the file: RFC 4180 quoting, CRLF line endings, UTF-8 with a
   BOM so Excel and Numbers detect the encoding, NULL as an empty field,
   binary as `0x…` hex. The path must be absolute in an existing folder and
   gets a `.csv` extension. A toast offers **Show in Finder**. Export covers
   all kept rows in the current sort.
6. **Permissions added:** `dialog:allow-save`,
   `opener:allow-reveal-item-in-dir`, `clipboard-manager:allow-write-text`.
7. **Database scoping (from user feedback).** A connection saved with a
   database shows only that database in the explorer and the tab's database
   picker; leave the field empty to browse every database. Running `USE db`
   in a MySQL or SQL Server tab moves the tab's picker to that database and
   keeps the same session.

## Verified

Unit tests cover CSV quoting, TSV copying, sorting with NULLs, export
files, paths and first-page bounds. Integration tests run 5,000-row and
row-limit queries against PostgreSQL and MySQL, check paging, numeric sort
and the exported file. 39 integration tests pass. The grid, sorting,
5,000-row scrolling, selection and inspector were checked in a browser
with the `?mock` backend.

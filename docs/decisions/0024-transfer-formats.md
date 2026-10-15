# 0024 — Copying and exporting a table

**Status:** accepted · **Date:** 2026-10-15 · Phase 12, `plan.md` §7b

With rows no longer kept in Rust (0023), copy and export have to read them
again. They stream: one page at a time, written straight out.

## Formats

CSV, TSV, JSON, a Markdown table, `INSERT` statements and `UPDATE`
statements. The toolbar's **Export…** lists them; the grid's context menu
exports CSV directly, which is what it always did.

## Decisions

1. **One page of memory, whatever the table's size.** `transfer::stream_table`
   reads 500 rows at a time and writes them out as it goes. A million-row
   export costs one page, not a million rows of string.
2. **Copy is capped; export is not.** The clipboard takes at most the
   connection's row limit, and says so. A file gets the whole table. Silently
   truncating either one would be worse than a number.
3. **CSV follows RFC 4180, and matches what results have always produced** —
   CRLF, quoting only where a comma, quote or newline requires it, doubled
   inner quotes. An apostrophe needs no quoting, and a test says so rather than
   leaving it assumed.
4. **JSON keeps every value a string and marks NULL as `null`.** Emitting
   numbers unquoted would risk turning a 20-digit identifier into a lossy
   float, which the grid deliberately never does. `null` still distinguishes
   SQL NULL from an empty string.
5. **SQL statements are meant to run.** Identifiers are quoted per engine,
   values are written as literals that the server casts (as 0015 does for
   staged edits), `UPDATE` uses the primary key as its `WHERE` clause and
   leaves those columns out of `SET`, and a table with no key says so instead
   of writing statements that would hit every row.
6. **The file extension follows the format.** `.md` for Markdown, `.json`,
   `.sql`, `.csv` for both delimited forms, and an extension already chosen by
   the user is left alone.
7. **`.xlsx` is not offered yet.** It needs `rust_xlsxwriter` or similar, a
   real dependency for one output format. Recorded here rather than added
   silently, and it stays on the list.

## A note on the test fixture

The SQL Server test container's default collation is Latin-1 and cannot store
U+2713 (`✓`), so the sample text stays inside Latin-1 and the column is
`NVARCHAR`. That is a limit of the container, not of the app: the app
round-trips whatever the server hands back. Getting this wrong in the other
direction — blaming the app for the mangling — would have been a false bug
report.

## Verified

- Unit: each format written from a row, including the escaping (`a,b "c"`
  becomes `"a,b ""c"""`), JSON string escaping, NULL and binary shapes, the
  Markdown pipe escape, and an `UPDATE` export refusing to run without a key.
- Integration: `*_streams_a_table_out` on PostgreSQL, MySQL and SQL Server
  writes a six-row table out as CSV and checks every row is present, the
  header, Unicode, quotes, a 20-digit number and an empty NULL; then a row
  scope with no header, the statement count, and that the copy cap is honoured.

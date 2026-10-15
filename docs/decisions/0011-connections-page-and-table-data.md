# 0011 — Connections page and table data

**Status:** accepted · **Date:** 2026-10-15 · From user feedback after Phase 8.

## Table data

**Open Data** on a table or view now uses the full results grid instead of
a fixed 100-row preview. The object tab gets its own database session
(like a query tab) and reads `SELECT * … LIMIT n` / `TOP (n)` with fully
quoted names, where n is "Rows kept per result" plus one row so the grid
can say when more rows exist. It scrolls, sorts, copies and exports to CSV
exactly like a query result. Closing the tab closes its session.

## Connections page

1. **Grouped by environment**, Production first, each group with a coloured
   left edge and a count. A search box filters by name, host, database,
   user or environment.
2. **Richer rows:** engine (name and engine colour), `user@host:port/db`,
   TLS state (verified, encrypted but unverified, or not encrypted, in
   warning colour when remote), SSH badge, and "last connected 2 h ago".
   Logos weren't used: they're trademarks and the engine colour already
   distinguishes them.
3. **Faster connecting:** double-click a row or press Return on it to
   connect or disconnect; ⌘⌫ deletes (with confirmation). **Switch
   Connection…** (⇧⌘K, File menu, Open Anything) lists connections by most
   recent use with fuzzy search, then selects and connects the one you pick.
4. **Connection URL:** paste a URL into the New Connection form to fill it
   in: `postgres://`, `postgresql://`, `mysql://`, `mariadb://`,
   `sqlserver://`, `mssql://`, with or without `jdbc:`. Credentials are
   URL-decoded; `sslmode` / `ssl-mode` / `encrypt` map to the TLS setting.
   The URL itself is never stored; a password from it goes to Keychain
   only if the connection is saved.
5. **Recent:** up to four most recently used connections appear as quick
   buttons at the top; one click connects.

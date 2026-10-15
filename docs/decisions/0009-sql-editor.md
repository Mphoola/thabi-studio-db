# 0009 — SQL editor and query execution

**Status:** accepted · **Date:** 2026-10-15

## Decisions

1. **CodeMirror 6, not Monaco.** MIT, modular (≈130 KB gzipped for what we
   use, loaded only when the workspace opens), native-feeling macOS text
   behaviour (it uses the browser's own selection and IME), and
   `@codemirror/lang-sql` has PostgreSQL, MySQL and SQL Server dialects.
   Monaco is ~5× larger, needs web workers and behaves like VS Code rather
   than a Mac text view. Colours come from our design tokens.
2. **Workspace tabs.** Query tabs and opened tables/views share one tab bar.
   Query tabs stay mounted (hidden) so each keeps its undo history. A dot
   marks SQL that isn't saved anywhere; closing such a tab, or a running
   one, asks first. Saving and recovery are Phase 9.
3. **One database session per query tab** (`src-tauri/src/queries.rs`),
   opened on the tab's first run in the tab's chosen database. Changing the
   database opens a new session. A tab runs one statement at a time.
4. **What Run runs.** The selection's statements, or the statement under the
   cursor; **Run All** runs every statement in order and stops at the first
   error. Splitting understands quotes, comments, PostgreSQL dollar quoting,
   MySQL backticks, SQL Server brackets and `GO` lines
   (`src/lib/sqlStatements.js`).
5. **Cancelling and timeouts.**
   - PostgreSQL and MySQL: a short second session sends
     `pg_cancel_backend(pid)` / `KILL QUERY id`; the tab's session (temp
     tables, transactions) is kept.
   - SQL Server: Tiberius can't cancel, so the tab's session is dropped,
     which makes the server stop. The result says the session was reset.
   - If a server-side cancel doesn't land within 5 s, the session is
     dropped too.
   - Timeout is a preference (Settings → Query timeout, default off) applied
     the same way.
6. **Never retried.** If the connection drops mid-statement the outcome is
   unknown, so the tab reports "connection lost" and the user decides.
7. **Asking before risky statements** (`src/lib/sqlSafety.js`): UPDATE or
   DELETE without WHERE, DROP and TRUNCATE always ask, naming the table;
   any data-changing statement on a **Production** connection asks "Run on
   Production?". This is a heuristic safeguard, not a guarantee.
8. **Results.** One result tab per statement with rows and time, or "N rows
   affected", or "No rows returned". Errors show the server's message and
   code with the SQL that ran. Up to 1,000 rows are kept per statement
   until the Phase 8 grid adds paging; only the first result set of a
   statement is shown and extra sets are counted. SQL Server doesn't report
   rows affected for plain statements yet.
9. **Native menu bar** (`src-tauri/src/menu.rs`) with File, Edit, View,
   Query, Window and Help. Items carry command ids and are forwarded to the
   frontend registry, so menu, shortcuts and Open Anything run the same
   code. ⌘W closes the tab, not the window.

## Shortcuts added

⌘T new query tab · ⌘W close tab · ⌘↵ run statement · ⇧⌘↵ run all ·
⌘. cancel query.

## Verified

Integration tests (`src-tauri/tests/queries.rs`): PostgreSQL cancel keeps
the session (temp table survives), MySQL cancel returns within seconds and
keeps session variables, SQL Server cancel resets the session and the next
run works, timeouts stop long queries, a tab rejects a second concurrent
statement, and changing database opens a new session. 37 integration tests
pass in total.

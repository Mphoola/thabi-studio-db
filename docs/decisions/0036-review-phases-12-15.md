# 0036 — Closing Phases 14 and 15 after review

**Status:** accepted · **Date:** 2026-10-23

## Context

A supervision pass over Phases 12–15 (`docs/reviews/2026-10-20-phases-12-15.md`)
found Phases 12 and 13 done, Phase 14 partly done with safety gaps, and
Phase 15 mostly not built. This records what branch `review-phases-12-15`
changed and the choices made along the way.

## Decisions

### Safety

- **Read-only is enforced in Rust.** `refuse_read_only(params)` guards every
  command that writes: row edits, table DDL, table actions, Run SQL File,
  database and schema actions, imports, restores, kills and user changes.
  SQL Server has no read-only session, so this check is its only guard.
  Backups are allowed on read-only connections; restores are not.
- **Drops name the object.** Dropping a column, index or key in the
  designer, a table, a database, a schema, a user, or restoring over a
  database all need the name typed.
- **No secret leaves Rust.** Native tools get passwords through
  `PGPASSWORD` or a 0600 MySQL option file that is removed when the job
  ends; tool output is scrubbed of the secret. User previews mask the
  password, and errors from user changes are scrubbed before they return.

### Phase 14

- **ALTER restates what it must.** MySQL `MODIFY`/`CHANGE` restate the
  whole column, so the current definition is read from `SHOW CREATE TABLE`
  first (`ddl/current.rs`); otherwise a rename or retype silently drops
  `NOT NULL`, the default and the comment. SQL Server keeps nullability on
  retype, sets defaults through a named constraint (`DF_table_column`) and
  comments through `MS_Description`.
- **Duplicate uses the engine's own copy.** `CREATE TABLE … LIKE` on MySQL,
  `(LIKE … INCLUDING ALL)` on PostgreSQL; copying data lists the columns
  (and uses `OVERRIDING SYSTEM VALUE` on PostgreSQL).
- **Constraints are described.** `describe` now returns constraints
  (foreign keys with their actions, checks, uniques), so the designer can
  show and drop multi-column foreign keys and checks, and the SQL export
  writes them faithfully.
- **Foreign key actions per engine.** SQL Server doesn't offer `RESTRICT`
  and InnoDB doesn't offer `SET DEFAULT`, so the form only lists what the
  engine accepts.
- **Routines** are listed and opened by identity (oid on PostgreSQL,
  object_id on SQL Server, name on MySQL). Editing one means opening its
  definition in a query tab and running it, like any statement.

### Phase 15

- **Import** reads CSV/TSV (delimiter detected, BOM stripped, UTF-8 or
  Latin-1) and JSON (array or JSON Lines), streams it, and inserts in
  batches inside one transaction with a savepoint per batch. A failing batch
  is retried row by row to name the bad line; skip-bad-rows is optional. On
  SQL Server, a conversion error dooms the transaction even inside
  `TRY…CATCH`, so values are pre-checked with `TRY_CAST` and `XACT_STATE()`
  is checked after each attempt.
- **Native tools are chosen to suit the server.** PostgreSQL's `pg_dump`
  must be at least the server's major version, so the newest installed is
  used and an older one is an error. For MySQL the same major version is
  preferred. `mysqldump` 9 fails on MySQL 8 when dumping routines, so with
  only that tool the backup leaves routines out and says so. `pg_restore`
  17+ issues `SET transaction_timeout`, which PostgreSQL 16 rejects; that
  one error is treated as harmless. SQL Server uses `BACKUP`/`RESTORE`
  on the server (path on the server, `SINGLE_USER` during restore).
- **Users** are managed through previewed SQL. PostgreSQL drops a role with
  `REASSIGN OWNED`, `DROP OWNED`, `DROP ROLE` in the current database, and
  the dialog says what that means. SQL Server creates a login and a database
  user together. Only a fixed list of privilege words is accepted.
- **The metrics board is not built.** It is marked optional in the plan
  and is left for later.

## New dependencies

- `csv` (Rust) — a correct CSV reader (quotes, embedded newlines) that
  streams; writing one by hand is a known source of bugs.
- `sqlx-mysql` with `chrono` — MySQL's prepared protocol returns dates and
  numbers in binary; decoding them needs typed access.
- tokio `process` and `io-util` features — to run the native tools as child
  processes and stream their output.

## Still open

- No test imports a million rows to prove flat memory (the reader streams
  and batches, but it isn't measured).
- No test proves autocomplete answers within 100 ms on a 500-table schema.
- The metrics board.

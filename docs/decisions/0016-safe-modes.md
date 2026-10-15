# 0016 — Safe modes per connection

**Status:** accepted · **Date:** 2026-10-15 · Phase 11, `plan.md` §7b

Safe mode is a setting on each saved connection, chosen by the user. It
replaces the single "this is Production, so ask" check that came with Phase 7
and 0015: the environment tag still colours the connection, but the safe mode
decides what Thabi Studio does before it sends anything.

## The four modes

| Mode                 | What it does                                                   |
| -------------------- | -------------------------------------------------------------- |
| `off`                | Nothing is asked. Destructive statements still ask (below).    |
| `confirm-writes`     | Asks before a statement that can change data or schema.        |
| `confirm-everything` | Asks before every statement runs.                              |
| `read-only`          | Refuses statements that can change data or schema; never asks. |

`safeMode` is stored on `connection_profiles` (migration `0007`) and returned
with the profile. New profiles follow their environment: Production starts on
`confirm-writes`, everything else on `off`. Existing Production profiles were
migrated to `confirm-writes`, so nobody loses a guard they already had.

## Decisions

1. **Destructive statements ask in every mode except `read-only`.** A
   `DROP`, `TRUNCATE`, or `UPDATE`/`DELETE` without a `WHERE` still shows the
   risk question even on `off`, because SQL can't be analysed reliably and
   this is the last cheap safeguard. `read-only` refuses them outright
   instead of asking.
2. **Production destructive statements ask for the database name to be
   typed.** `confirm()` grew a `requireText` option, and `ConfirmHost` shows
   a text field with Confirm disabled until the name matches exactly. The
   connection and database are named in every confirmation, as the plan
   requires.
3. **Read-only is enforced twice.** The app classifies the statements and
   refuses before anything is sent (`writeBlockFor`); Rust also asks the
   server, per session, with `SET SESSION TRANSACTION READ ONLY` (MySQL) or
   `SET default_transaction_read_only = on` (PostgreSQL). `Session::connect`
   runs it, so the main session and every query-tab session are covered —
   `ConnectParams.read_only` is what the profile sets.
4. **SQL Server has no session equivalent, so only the app stops writes.**
   The Safe Mode hint in the connection form and the read-only note on the
   data view both say so rather than implying a guarantee. The integration
   test `sqlserver_has_no_read_only_session` records the difference instead of
   hiding it.
5. **Staged edits follow the same mode.** `confirm-writes` and
   `confirm-everything` ask before Apply; deletes always ask, with the table
   name typed on Production. A read-only connection disables editing in the
   grid entirely (`editable`), so changes can't be staged in the first place.
6. **The connection pill shows the mode** as an icon — a shield for the
   confirming modes, a lock for read-only — with the meaning in its tooltip.
   `off` shows nothing.

## Verified

- Unit: safe-mode classification, `writeBlockFor`, and apply confirmations
  (`tests/frontend/sqlSafety.test.js`); `safe_mode` round-trips through
  create, update, duplicate and connect params, Production migrates to
  `confirm-writes`, an unknown mode is rejected by field
  (`profiles::repository`).
- Integration: `postgres_read_only_session` and `mysql_read_only_session`
  prove the server refuses the write while reads keep working;
  `sqlserver_has_no_read_only_session` pins the documented difference.

## Not done here

Phase 11 continues with connection groups and colours, `~/.ssh/config`,
"ask every time" secrets, startup commands, mutual TLS, per-connection
timeouts, import/export, multiple windows and reconnect. Each is recorded in
its own decision file.

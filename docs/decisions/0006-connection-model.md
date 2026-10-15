# 0006 — Connection model, TLS and errors (after the Phase 3–4 review)

**Status:** accepted · **Date:** 2026-10-15 · Supersedes parts of 0005.

Context: `docs/reviews/2026-10-15-phase-3-4-review.md`.

## Decisions

1. **One live connection is one database session.** No pools. Each engine
   has a `*Session` type (`database/`) that owns a single `PgConnection`,
   `MySqlConnection` or Tiberius `Client`, so `USE`, `SET`, transactions
   and temp tables behave as written. A `tokio::Mutex` keeps statements on
   a session in order. Query tabs (Phase 7) will each open their own session.
2. **Driver contract.** `DatabaseSession`: connect, server info,
   capabilities, ping, list databases, execute (bounded rows), close.
   `Session` dispatches to the engine. Capabilities state what isn't
   possible yet (cancellation everywhere; rows affected on SQL Server).
3. **Values travel as text.** Postgres uses the simple query protocol and
   MySQL the text protocol, so every value arrives as the server's text;
   SQL Server values are converted per type. Large integers and decimals are
   never turned into JavaScript numbers. Binary is a length plus hex preview.
4. **TLS is a per-connection choice:** Verify certificate (default, with an
   optional custom CA file), Encrypt without verifying, or Off. A new
   connection to `localhost`/`127.0.0.1`/`::1` starts with Off until the
   user picks; nothing is ever downgraded automatically. The CA file is
   picked with the native dialog (`tauri-plugin-dialog`, `dialog:allow-open`
   only) and checked in Rust (absolute path, a file, ≤ 1 MB).
5. **Errors carry the cause.** `AppError` has an owned `message` for the
   user plus sanitised `detail` (the server's text) and `code`. DNS is
   checked first; refused, unreachable, timeout, closed-by-server, TLS,
   TLS-not-supported, authentication, unknown database and server errors
   each have their own kind and message. Passwords are scrubbed from detail.
6. **Test Connection uses the form's values** (`test_connection_input`),
   reusing an edited profile's saved password unless a new one is typed or
   removal is ticked. It reports "Connected to PostgreSQL 16.4 in 38 ms."
7. **Lifecycle states:** Connecting, Connected, Closing; no entry means
   disconnected. Disconnect during connect cancels it and closes whatever
   opened. Disconnect always removes the entry. A dropped connect call
   clears its reservation on a background task (no `try_lock`). Connections
   close on app exit, after 60 minutes unused, or when a 5-minute health
   check fails; the frontend hears about the last two via a
   `connection-closed` event.
8. **Commands are async.** SQLite and Keychain work runs on a blocking
   thread (`AppState::blocking`), never on the main thread.
9. **Migrations** are SQL files run once each, in order, in a transaction.
   Shipped migrations are never edited. 0003 adds `tls_ca_path`.
10. **Keychain through `keyring-core` + `apple-native-keyring-store`.** Same
    Keychain items as before (service `com.thabi.studio`), so saved
    passwords keep working.
11. **Hosts** accept underscores (Docker Compose names) and strip IPv6
    brackets. `host\instance` is rejected with a message to use the port.

## Known limitations

- Query cancellation isn't implemented yet. Postgres and MySQL sessions
  record their backend id for Phase 7; Tiberius has no cancellation.
- Only the first row-returning result set is kept; others are counted.
- SQL Server reads rows past the limit and drops them (Tiberius needs the
  stream drained). Paging arrives in Phase 8.
- SSH-enabled profiles are refused until Phase 5.

## Verified

`npm run test:integration` runs 23 tests against PostgreSQL 16, MySQL 8.4
and SQL Server 2022 in Docker: connect and version, lossless values (big
integers, decimals, NULL, Unicode), session state across statements,
truncation, rows affected, server errors with detail, wrong password,
unknown database, refused port, and TLS failures.

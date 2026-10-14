# Review: Phase 3 (committed) and Phase 4 (in progress)

Reviewed 2026-10-15 against `plan.md`, `docs/design/workspace.md` and the
decision records. `npm run check` passes (28 frontend and 21 Rust tests).
Phase 4 is uncommitted and, as `docs/decisions/0005` says, unfinished.

## What's right

- Secrets live only in Keychain behind random opaque references; SQLite holds
  booleans, and commands never return secret values. Tests prove it.
- Validation runs in both Vue and Rust, with field-level errors.
- Profile CRUD, duplicate, delete and credential replacement handle
  Keychain/SQLite partial failures carefully, with good tests.
- SSH profiles are refused instead of silently connecting directly.
- Docs are honest: Phase 4 is marked in progress, limitations are written down.

## Must fix

### 1. Local databases can't connect (TLS is verify-full only)

`connections.rs` rejects any `tls_mode` other than `verify-full`, and the form
has no TLS option. A Docker Postgres has no TLS at all, and Docker MySQL and
SQL Server use self-signed certificates, so connecting to a typical local or
dev database fails.

Fix: per-connection TLS setting in the form:
- **Verify certificate** (default): current behaviour.
- **Verify with custom CA…**: pick a CA file (for internal CAs).
- **Encrypt, don't verify**: allowed, labelled with a warning.
- **Off**: allowed, labelled; the default suggestion only for `localhost`.
Never change the mode silently. This is what plan.md principle 5 asks for.

### 2. A connection pool is the wrong model for a query editor

MySQL and Postgres open a pool of 5. In an editor, `USE db`, `SET`,
`BEGIN … COMMIT`, temp tables and session variables must stay on the same
server session, and cancelling a query needs to know which backend runs it.
With a pool, consecutive statements can land on different connections.

Fix: one dedicated connection per editor session (`PgConnection`,
`MySqlConnection`, Tiberius `Client`), plus at most one separate connection
for explorer metadata. Define the shared driver trait now (connect, ping,
close, list databases, execute, cancel, capabilities) so Phase 6–7 build on it.

### 3. Error messages lose the cause, and some are wrong

- Every `sqlx::Error::Database` becomes "The database rejected the login",
  including "database does not exist" and permission errors.
- Refused port, unknown host and unreachable host all read the same.
- `AppError.message` is `&'static str`, so the server's own message can never
  be shown. Phase 7 (query errors) needs it, and the agreed copy
  ("The server refused the connection on port 5432…") can't be produced.

Fix: give `AppError` an owned message plus optional sanitised detail
(`code`, `server_message`). Map `io::ErrorKind::ConnectionRefused`, DNS
failure, timeout, TLS, auth (PG `28P01`/`28000`, MySQL `1045`, MSSQL `18456`)
and unknown database (PG `3D000`, MySQL `1049`, MSSQL `4060`) separately.
Redact anything that could contain the password.

### 4. You can only test a connection after saving it

`ConnectionTestAction` is disabled for unsaved profiles. Users expect to fill
in the form, press Test, then save.

Fix: a `test_connection_input` command that takes the form values and typed
password, validates them in Rust, tests, and stores nothing. Show
"Connected to PostgreSQL 16.4 in 38 ms." on success (server version + time).

## Should fix

5. **Blocking commands on the main thread.** Profile and settings commands are
   plain `fn` commands, which Tauri runs on the main thread. They take the
   SQLite mutex and call Keychain, which can be slow or show a macOS prompt,
   so the window can freeze. Make them `async` and use
   `tauri::async_runtime::spawn_blocking`.
6. **Fragile migration runner.** `storage.rs` splits SQL on `;` and parses
   `ALTER TABLE … ADD COLUMN` text to skip columns that already exist. That
   exists because migration 0002 was changed after it had been applied. Run
   each migration file with `execute_batch` inside a transaction, and never
   edit a migration once it has run.
7. **Connection lifecycle edge cases.** `disconnect_profile` removes the entry
   before `Arc::try_unwrap`; if anything else holds the connection it's
   dropped from the registry without being closed and an error is returned.
   The reservation's `Drop` uses `try_lock`, which can leave a profile stuck
   in "connecting". Plan.md asks for explicit states (disconnected,
   connecting, connected, closing, error); model them and make connect
   cancellation-safe.
8. **Host validation is too strict and slightly inconsistent.** It rejects
   underscores (Docker Compose names like `my_db`) and SQL Server
   `host\instance`, and accepts `[::1]` but then passes the brackets to
   `TcpStream::connect`, which fails.
9. **`keyring` 4 in v1-compatibility mode.** The crate's own docs say apps
   should depend on `keyring-core` plus `apple-native-keyring-store`.
   Works today; switch before release.
10. **Design drift from the agreed workspace doc:**
    - Copy was weakened: "Connection test succeeded." instead of version and
      time; "Save a connection profile" instead of "Connect to your first
      database" ("profile" is internal jargon).
    - The Production red strip and Production confirmation were never built.
    - Saving shows a success toast; the dialog closing already says that.

## Small

- `last_connected_at` is never written.
- `credentials.rs` test uses a no-op store and proves nothing.
- `commands/connection_profiles.rs` is ~1,080 lines; split into
  validation, repository and commands.
- `ConnectionProfileDialog.vue` is at 248 of the 250-line limit; move the
  database fields into their own component before adding TLS options.
- Phase 4 still lacks: list databases/schemas, execute query, result metadata,
  capabilities, idle disconnect, and integration tests against real
  MySQL/Postgres/SQL Server (e.g. Docker Compose + env-provided credentials).

## Suggested order

1, 3 and 4 together (they all touch connect/test), then 2 (driver trait and
sessions), then 5–8, then finish Phase 4's remaining items with integration
tests. 9 and 10 can go alongside.

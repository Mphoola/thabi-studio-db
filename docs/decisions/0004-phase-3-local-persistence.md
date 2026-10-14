# 0004 — Phase 3: local persistence and connection profiles

**Status:** accepted · **Date:** 2026-10-14

## Decisions

1. **SQLite belongs to the Rust side.** `rusqlite` opens
   `thabi-studio.sqlite3` in Tauri's application data directory. Schema
   changes are versioned with SQLite's `user_version` and checked-in SQL
   migration files. No database plugin or new frontend capability is needed.
2. **OS credential storage uses `keyring`.** On macOS it selects Keychain
   Services. Connection metadata stores only a random opaque reference and
   booleans indicating which credentials were saved; a JSON bundle containing
   the database password and optional SSH password is stored as a single
   Keychain item. Commands never return credential values to Vue.
3. **Cross-store changes are explicit.** Credential updates write a new
   Keychain item before switching the SQLite reference, then remove the old
   item. If old-item cleanup fails after a successful update, the response
   reports a warning. Failed profile creation removes its newly written
   credential or reports that manual Keychain cleanup may be needed.
4. **Only password authentication is offered for now.** Database and SSH
   passwords are supported and stored securely. SSH private-key authentication
   and actual tunnel setup are deferred to Phase 5; a profile marked for SSH
   does not create a tunnel in this phase.
5. **Profile selection is not a live connection.** The selected profile ID is
   a non-secret app setting. The UI explicitly says “Not connected”; the
   connection-test control stays disabled until Phase 4 supplies a real driver.
   Deleting a profile removes its saved credentials and clears selection. No
   live connection exists in this phase; disconnect-on-delete will be wired
   with the Phase 4 connection lifecycle.
6. **Existing shell preferences migrate once.** Theme, sidebar width, and
   sidebar-open state move from the temporary `localStorage` keys to
   `app_settings` on first launch after this phase. Later preference changes
   write directly to SQLite.

## Dependencies

- `rusqlite` with bundled SQLite provides a small embedded local database and
  avoids relying on a system SQLite installation.
- `keyring` delegates secret storage to the platform credential store, using
  the native macOS Keychain without adding a Tauri plugin or permission.

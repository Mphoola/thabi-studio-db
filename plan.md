# Thabi Studio — Development Plan

**Product:** Thabi Studio  
**Product type:** Native desktop database client  
**Platforms:** macOS, Windows and Linux (cross-platform from the start; developed on macOS)  
**Stack:** Tauri 2, Vue 3, JavaScript, Rust  
**Initial database engines:** MySQL, PostgreSQL, Microsoft SQL Server  
**Primary use:** Personal daily development work, with builds shared with developer friends  
**Document purpose:** Phased implementation plan: the MVP (Phases 1–10) and the extended phases that make it a full-featured client (Phases 11–15)

---

## 1. Product vision

Thabi Studio is a polished, local-first desktop application for connecting to, exploring, querying, and managing MySQL, PostgreSQL, and Microsoft SQL Server databases from one workspace.

It should feel like a carefully designed native developer tool on each platform (macOS, Windows and Linux): fast, calm, keyboard-friendly, and reliable. The app connects directly from the user's computer to database servers. **No Thabi-hosted backend, user account, telemetry service, or cloud dependency is required for the MVP.**

The goal is a **full-featured database client** that developers can use every day instead of TablePlus or DataGrip: connect anywhere, browse and edit data, write and understand SQL, change schemas, move data in and out, and look after servers, all with the same calm design and the same safety guarantees.

It is built in two stages:

1. **MVP (Phases 1–10):** reliable connections, safe query execution, schema exploration and a high-quality results grid, released for personal use. Phase 9b added inline data editing once that base was proven.
2. **Full-featured (Phases 11–15):** safety modes and connection power features, table data power-ups, a schema-aware SQL editor, schema and object management, and import/export/backup/administration. See §7b.

Reliability and safety come before breadth at every stage: a feature that can change data ships only with a reviewable preview, a confirmation on production, and integration tests on all three engines.

### MVP success criteria (Phases 1–10)

A user can:

1. Install and launch Thabi Studio on macOS, Windows and Linux.
2. Create, edit, duplicate, test, and delete saved connection profiles.
3. Connect to local or remote MySQL, PostgreSQL, and Microsoft SQL Server databases.
4. Connect directly over TCP/TLS or through an SSH tunnel.
5. Browse available databases, schemas, tables, views, columns, and indexes where supported.
6. Open multiple SQL editor tabs, execute queries, cancel queries where the driver supports cancellation, and view results and errors.
7. Inspect large result sets using pagination or bounded result retrieval rather than loading unlimited rows into memory.
8. Copy selected cells/rows and export query results to CSV.
9. View query history and save reusable SQL snippets/queries.
10. Close connections and SSH tunnels cleanly, with credentials stored securely.

### Full-featured success criteria (Phases 11–15)

On top of the MVP, a user can:

1. Choose a safe mode per connection (off, confirm writes, confirm everything, read-only), group and colour connections, use `~/.ssh/config` hosts, and work in several windows.
2. Page through and sort whole tables on the server, see a row as a form, hide and reorder columns, follow foreign keys, and copy or export data as CSV, JSON, SQL, Markdown or Excel.
3. Edit data from table tabs and from single-table query results, with staged changes, SQL review and one transaction.
4. Write SQL with schema-aware autocomplete, formatting, bound query parameters, query plans, multiple result sets and split panes.
5. Create and change tables, columns, indexes, keys, views, routines and triggers through a designer that shows the DDL before running it, and see a schema as an ER diagram.
6. Import CSV/JSON, export and run SQL dumps, back up and restore with native tools, manage users and permissions, and see and stop server sessions.

---

## 2. Scope boundaries

### Included in MVP

- Desktop packaging for macOS, Windows and Linux with Tauri 2.
- Vue 3 + Vite frontend written in JavaScript.
- Rust backend for database drivers, SSH tunnelling, connection lifecycle, and sensitive operations.
- MySQL, PostgreSQL, and Microsoft SQL Server support.
- Local connection profiles and application preferences.
- OS-backed secure storage for database and SSH secrets.
- Direct TCP connections and SSH tunnels.
- TLS settings with certificate verification enabled by default.
- Connection testing and useful, sanitised error messages.
- Database/schema/table explorer.
- SQL editor with syntax highlighting, multiple tabs, run-selection/run-query actions, and keyboard shortcuts.
- Query results grid with loading, empty, error, and pagination states.
- Query history and saved queries.
- CSV export and clipboard copy.
- Basic safety protections, tests, logging that excludes secrets, documentation, and distributable development builds.

### Out of scope for the MVP, planned for later phases

These were deliberately left out of Phases 1–10. Most are now planned in §7b; the phase is noted. Do not start them before the MVP ships.

- User accounts, cloud sync, team workspaces, hosted services, or a licensing server.
- AI query generation or AI assistant features.
- Visual query builder (not planned).
- Full ERD/schema diagramming (read-only ER diagram: Phase 14).
- Cross-database schema/data comparison and migration tooling (not planned).
- Database backup/restore orchestration (Phase 15).
- Import wizard for many file formats (CSV/TSV/JSON and SQL dumps: Phase 15).
- Table designer capable of changing arbitrary schema objects (Phase 14).
- Advanced data editing with bulk insert/update/delete (Phase 9b, extended in Phase 12).
- Stored procedure/function editors (Phase 14); a debugger is not planned.
- Replication and performance profiling (not planned); explain-plan visualisation (Phase 13); server activity and a metrics board (Phase 15).
- Plugin marketplace (not planned).
- Automatic updates unless a safe and practical release mechanism is implemented after the core app is stable.

Inline data editing was added as a separately scoped milestone (Phase 9b) after connection and query reliability were proven.

---

## 3. Engineering principles

1. **Rust owns database access.** The Vue frontend must never connect directly to a database or receive raw credentials unnecessarily.
2. **Use a driver abstraction, not a lowest-common-denominator SQL abstraction.** Common UI operations share an interface, while engine-specific metadata queries and capabilities remain separate.
3. **Do not assume all SQL dialects are equivalent.** Use driver-specific implementations for metadata, quoting, cancellation, transactions, and pagination where necessary.
4. **Use parameterised queries for app-generated SQL.** User-entered SQL is executed as authored, with clear display of the active connection and database.
5. **Never silently weaken security.** TLS certificate verification is on by default. Any insecure override must be explicit, clearly labelled, and scoped to that connection.
6. **Never log secrets.** Redact passwords, connection strings, SSH private-key material, tokens, and sensitive parameters.
7. **Bound memory usage.** Do not load an unbounded query result into frontend state.
8. **Keep UI responsive.** Long-running operations run off the UI thread and provide progress/loading states and cancellation when possible.
9. **Design for recoverability.** Distinguish connection failures, SQL errors, timeouts, cancellation, and unsupported capabilities.
10. **Keep components small and reusable.** Target Vue components under 250 lines. Split larger components by responsibility rather than mechanically extracting fragments.
11. **Prefer JavaScript, not TypeScript,** unless a specific dependency or API makes TypeScript unavoidable.
12. **Test real driver behaviour early.** A mock is useful for UI tests, but cannot replace integration tests against all three database engines.

---

## 4. Proposed architecture

```text
Thabi Studio
├── Vue 3 + Vite frontend
│   ├── App shell / workspace layout
│   ├── Connection manager
│   ├── Database explorer
│   ├── SQL editor tabs
│   ├── Query results grid
│   ├── Query history / saved queries
│   └── Settings / dialogs / notifications
│
├── Tauri IPC boundary
│   └── Typed-by-convention command/request and response DTOs
│
├── Rust application services
│   ├── Connection registry and lifecycle
│   ├── Credential-store adapter
│   ├── SSH tunnel manager
│   ├── Query execution service
│   ├── Query result paging / cancellation
│   ├── History and saved-query repository
│   └── Sanitised diagnostics
│
├── Database driver abstraction
│   ├── MySQL implementation
│   ├── PostgreSQL implementation
│   └── SQL Server implementation
│
└── Local app data
    ├── SQLite: profiles metadata, preferences, history, saved queries
    └── OS secure credential store: passwords, private-key secrets/passphrases
```

### Important implementation note

The diagram describes responsibilities, not a requirement to build a large framework. Start with the smallest modular structure that supports the use cases. Avoid a generic plugin system until a second or third implementation proves it is necessary.

### Database driver feasibility spike

Before committing to the full architecture, build a small Rust proof of concept for each engine.

- **MySQL:** evaluate `sqlx` MySQL support or another actively maintained driver that meets licensing and platform needs.
- **PostgreSQL:** evaluate `sqlx` PostgreSQL support.
- **SQL Server:** evaluate `tiberius` and its current compatibility, TLS behaviour, authentication options, data type coverage, async runtime requirements, and maintenance status.
- Check crate versions, licensing, release activity, supported Rust toolchain, macOS (Apple Silicon and Intel), Windows and Linux builds, and whether the chosen driver can use a TCP stream supplied by the SSH tunnel implementation.
- Confirm how each driver exposes column metadata, null values, binary values, decimal values, timestamps, and cancellation.
- Do not assume that SQL Server authentication or SSH tunnelling works just because a basic local connection succeeds.

Record the outcome and selected crate versions in `docs/architecture/database-drivers.md`. If one driver cannot meet a required feature, decide explicitly whether to replace it, limit a capability, or defer that capability. Do not hide a missing capability behind a fake success response.

---

## 5. Repository structure

Adapt to the actual scaffold created by the selected Tauri template. Do not overwrite existing generated configuration without inspecting it first.

Suggested structure:

```text
thabi-studio/
├── src/                          # Vue frontend
│   ├── assets/
│   ├── components/
│   │   ├── app/
│   │   ├── connections/
│   │   ├── explorer/
│   │   ├── editor/
│   │   ├── results/
│   │   ├── dialogs/
│   │   └── common/
│   ├── composables/
│   ├── layouts/
│   ├── pages/
│   ├── stores/
│   ├── lib/
│   │   ├── tauri/
│   │   ├── formatting/
│   │   └── validation/
│   ├── styles/
│   └── App.vue
├── src-tauri/
│   ├── capabilities/
│   ├── icons/
│   ├── src/
│   │   ├── commands/
│   │   ├── database/
│   │   │   ├── mod.rs
│   │   │   ├── traits.rs
│   │   │   ├── mysql/
│   │   │   ├── postgres/
│   │   │   └── sqlserver/
│   │   ├── connections/
│   │   ├── credentials/
│   │   ├── ssh/
│   │   ├── history/
│   │   ├── errors/
│   │   └── lib.rs
│   └── Cargo.toml
├── tests/
│   ├── frontend/
│   ├── rust/
│   └── integration/
├── docs/
│   ├── architecture/
│   ├── decisions/
│   └── screenshots/
├── scripts/
├── CLAUDE.md
├── README.md
└── plan.md
```

Use names and locations consistent with the real scaffold. Do not create empty directories just to match this example.

---

## 6. Data and state design

### Local SQLite tables

Use migrations for the application's SQLite database. Keep secrets out of SQLite.

#### `connection_profiles`

Suggested fields:

- `id` — stable UUID/string identifier.
- `name` — user-defined display name.
- `driver` — `mysql`, `postgres`, or `sqlserver`.
- `host`.
- `port`.
- `database_name` — optional initial database.
- `username`.
- `auth_mode` — supported authentication mode identifier.
- `tls_mode` — validated enum, secure by default.
- `ssh_enabled`.
- `ssh_host`, `ssh_port`, `ssh_username`, `ssh_auth_mode`.
- `credential_key` — opaque key used to retrieve secrets from OS secure storage.
- `created_at`, `updated_at`.
- `last_connected_at` — optional.

Do not store plaintext passwords, database connection URLs containing passwords, SSH private keys, or key passphrases in SQLite. Avoid storing private-key contents in a normal file unless a deliberate, documented secure-storage design is approved.

#### `query_history`

- `id`.
- `connection_profile_id`.
- `query_text`.
- `started_at`, `duration_ms`.
- `status` — success, error, cancelled, or timeout.
- `rows_returned` where available.
- `error_summary` — sanitised; no secrets.
- Optional database/schema context.

Consider a history retention limit and a setting to disable history. SQL can contain literals or sensitive information, so history must be treated as user data. Provide clear-history controls and document where it is stored.

#### `saved_queries`

- `id`.
- `name`.
- `query_text`.
- Optional `connection_profile_id`.
- Optional notes/tags.
- `created_at`, `updated_at`.

#### `app_settings`

Store only non-secret preferences: theme, editor font size, result page size, history retention, default connection behaviour, and layout preferences.

### Runtime state

Use a frontend store for UI state only: open tabs, selected connection, active editor, explorer expansion, visible results, and dialogs. Rust owns live connections, tunnels, query tasks, and cancellation handles.

A saved connection profile is not the same as a live database connection. Track their states separately.

---

## 7. Phased implementation plan

Complete phases in order. Each phase must pass its exit criteria before starting the next. Claude Code should inspect the repository and current state before implementing a phase, update this plan's progress checklist, and avoid rewriting working code without a clear reason.

### Phase 1 — Project scaffold and engineering baseline

**Goal:** Establish a clean, runnable Tauri/Vue/Rust project.

Tasks:

- Inspect the repository, existing files, installed Node.js/Rust toolchains, and the current Tauri template before making changes.
- Create the project using the official Tauri 2 project creation flow if no suitable scaffold exists.
- Configure Vue 3 + Vite and JavaScript.
- Confirm `cargo check`, frontend build, and Tauri development launch work (on the development Mac; CI covers Windows and Linux).
- Add formatting/lint scripts supported by the chosen scaffold.
- Add a basic app shell, error boundary/fallback view, and consistent notification/dialog primitives.
- Establish the theme tokens, typography, spacing, borders, focus styles, and light/dark theme strategy.
- Create `CLAUDE.md` with project context, coding rules, commands, and security constraints.
- Add a README with prerequisites and local development instructions.
- Record dependency versions and key architectural decisions.

**Deliverables:**

- Running desktop shell.
- Repeatable development/build commands.
- Initial project structure.
- `CLAUDE.md`, `README.md`, and architecture decision notes.

**Exit criteria:**

- A clean checkout can install dependencies and run the app using documented steps.
- The app opens without console or Rust errors.
- Lint/build/check commands pass.
- No database functionality is faked or implied yet.

### Phase 2 — Product shell and design system

**Goal:** Build the premium, minimal desktop workspace before complex database features.

Tasks:

- Create the main workspace layout with navigation/sidebar, content area, and status/connection indicators.
- Design an empty state that guides users to create their first connection.
- Build reusable buttons, icon buttons, text inputs, selects, checkboxes, switches, tabs, dropdowns, tooltips, modal/dialogs, toasts, loading indicators, empty states, and error states.
- Ensure keyboard focus is visible and controls have accessible labels.
- Add responsive layout behaviour for smaller windows.
- Add theme tokens in one place; avoid arbitrary hard-coded colours throughout components.
- Use native-feeling spacing and restrained colour accents. Avoid visual clutter and oversized cards.
- Establish an icon set and consistent icon sizing.
- Add keyboard shortcuts infrastructure without binding shortcuts that conflict with platform conventions (⌘ on macOS, Ctrl on Windows and Linux).

**Deliverables:**

- Main app shell.
- Reusable component set.
- Design tokens and theme support.
- Empty, loading, error, and disconnected states.

**Exit criteria:**

- Layout works at typical MacBook window sizes and can be resized.
- All controls are keyboard accessible.
- Components use shared design tokens.
- Vue components remain under the 250-line target unless a documented exception is approved.

### Phase 3 — Local persistence and connection profiles

**Goal:** Let users manage saved connection profiles without connecting yet.

Tasks:

- [x] Add versioned SQLite migrations for connection profile metadata and app settings.
- [x] Add create, edit, duplicate, delete, list, and select profile operations.
- [x] Build a connection form with MySQL, PostgreSQL, and SQL Server port defaults; offer only supported password authentication.
- [x] Validate required fields, ports, hostnames, database names, and supported authentication options in both frontend and Rust.
- [x] Provide show/hide password controls and safe password-field behaviour.
- [x] Integrate OS-backed secure storage for database and SSH passwords.
- [x] Store opaque secret references rather than secret values in SQLite.
- [x] Handle credential update, removal, missing-secret, and secure-store access failures.
- [x] Add environment labels and colours to profiles.
- [x] Confirm profile deletion, remove associated credentials, and clear selected-profile state. No live connections exist in this phase; disconnect integration is deferred until Phase 4 introduces connection lifecycle.
- [x] Show a disabled “Test Connection” state; do not report success before a real driver exists.

**Deliverables:**

- [x] Persistent connection manager.
- [x] Database-specific connection forms.
- [x] Secure credential-store abstraction.
- [x] SQLite migrations and repository tests.

**Exit criteria:**

- [x] Profiles and selection survive reopening the SQLite database.
- [x] Secrets are not present in SQLite profile records or command responses; errors are sanitised and credential values are not logged.
- [x] Invalid forms show clear, field-specific errors.
- [x] CRUD and forward-migration tests pass, including the in-progress v1 schema upgrade.
- [x] No fake connection-success state is presented.

### Phase 4 — Database driver proof of concept and connection lifecycle

**Goal:** Prove all three engines work before building advanced database features.

Tasks:

- Implement the shared database driver contract for connect, disconnect, health check, list databases, list schemas/tables where practical, execute query, retrieve result metadata, and report capabilities.
- Create MySQL implementation and integration tests.
- Create PostgreSQL implementation and integration tests.
- Create SQL Server implementation and integration tests.
- Evaluate and document the exact Rust crates and versions selected.
- Implement connection timeout, connect timeout, idle disconnect strategy, and predictable resource cleanup.
- Model connection lifecycle states: disconnected, connecting, connected, reconnecting if supported, closing, and error.
- Return structured, sanitised errors to the frontend.
- Keep live connections in Rust behind stable connection IDs. Do not send connection objects or secrets to Vue.
- Integrate profile deletion with the connection lifecycle so an active connection is closed before its saved credentials are removed.
- Add connection testing through the real selected driver.
- Verify connections against local development databases and at least one remote environment per supported engine where available.
- Document required environment variables or test configuration without committing credentials.
- [x] Evaluate and document the selected Rust drivers, versions, runtime, and TLS features (`docs/decisions/0005`, `0006`).
- [x] Shared driver contract (`DatabaseSession`): connect, ping, list databases, execute with bounded rows and column metadata, capabilities, close.
- [x] One dedicated session per connection (no pools), so session state and future cancellation work.
- [x] Per-connection TLS: verify (default, optional custom CA), encrypt without verifying, off. Never downgraded automatically.
- [x] Specific, sanitised errors: host not found, refused, unreachable, timeout, closed, TLS, TLS unsupported, authentication, unknown database, server error with detail and code.
- [x] Lifecycle states (connecting, connected, closing), cancellation-safe connect, close on exit, idle disconnect after 60 minutes, health check every 5 minutes.
- [x] Profile edits and deletion close the live connection first.
- [x] Test Connection on unsaved form values, showing server version and time.
- [x] Integration tests against PostgreSQL 16, MySQL 8.4 and SQL Server 2022 (`npm run test:integration`, Docker, generated throwaway credentials).
- [ ] Verify against at least one remote server per engine (needs servers you can reach).

**Deliverables:**

- Three functioning driver adapters.
- Connection lifecycle service.
- Real connection test feature.
- Driver feasibility notes and integration tests.

**Exit criteria:**

- Each engine can establish a real connection, run a simple query, and close cleanly.
- Failures such as wrong password, unreachable host, refused port, timeout, and invalid TLS configuration produce useful errors without leaking secrets.
- macOS Apple Silicon build works; Windows and Linux builds pass CI.
- SQL Server limitations, if any, are documented explicitly.

### Phase 5 — Remote connectivity, TLS, and SSH tunnelling

**Goal:** Support secure local and remote database access.

Tasks:

- [x] Implement direct TCP connectivity for each supported driver.
- [x] Configure TLS and certificate verification with secure defaults, subject to driver capabilities.
- [x] Support explicit trust configuration where appropriate (custom CA file); clearly warn about disabling certificate verification.
- [x] Implement SSH local port forwarding with a maintained Rust SSH library (`russh`) after checking platform compatibility.
- [x] Support SSH host, port, username, and password, key file and agent authentication.
- [x] Verify SSH host keys; do not silently accept unknown or changed host keys.
- [x] Support SSH key selection/passphrase handling only through a safe, documented design (`docs/decisions/0007-ssh-tunnels.md`).
- [x] Map the local forwarded endpoint (127.0.0.1 only) to the selected database connection without exposing the remote database publicly.
- [x] Tie tunnel lifetime to the associated database connection; close the tunnel on disconnect, profile deletion, app shutdown, and failed connection setup.
- [x] Handle tunnel conflicts (random local ports), SSH authentication errors, host-key mismatch, network loss (keepalive + health checks), and remote endpoint failure.
- [x] Add test coverage for connection and tunnel cleanup.
- [x] Document that the user's existing VPN/private network can be used where applicable.

**Deliverables:**

- [x] Direct remote connections.
- [x] TLS configuration.
- [x] SSH tunnel management.
- [x] Clear connection security indicators (status bar).

**Exit criteria:**

- [ ] A user can connect to a remote database directly when network access permits. (Code path is the same as local; not yet checked against a real remote server.)
- [x] A user can connect to a private database through an SSH tunnel (Docker bastion, all three engines).
- [x] TLS and SSH host-key failures are surfaced rather than bypassed.
- [x] No tunnel is left running after the owning connection is closed.
- [x] Secrets and private key material never appear in logs or errors.

### Phase 6 — Database explorer and metadata

**Goal:** Browse database structure reliably.

Tasks:

- [x] Build a lazy-loaded tree for connections, databases, schemas, tables, and views.
- [x] Load child nodes on demand rather than querying every object at once.
- [x] Implement driver-specific metadata queries.
- [x] Display columns, types, nullability, primary keys, indexes, and other metadata where supported.
- [x] Distinguish unsupported features from empty results.
- [x] Add refresh for the selected node and refresh-all where reasonable.
- [x] Add object context menus for supported actions, starting with “Open data” and “Copy name”.
- [x] Quote/escape identifiers using database-specific rules for any generated SQL.
- [x] Display loading, permission-denied, stale-connection, and empty-schema states.
- [x] Avoid blocking the UI while metadata is fetched.
- [x] Add metadata tests for names containing spaces, reserved words, mixed case, and non-ASCII characters.

**Deliverables:**

- [x] Schema explorer.
- [x] Table/view structure panel.
- [x] Metadata adapters for each database.

**Exit criteria:**

- [x] Users can navigate database objects for all three engines.
- [x] Explorer requests are lazy. Cancelling an in-flight metadata request isn't implemented (requests are short); see `docs/decisions/0008-explorer.md`.
- [x] Unsupported metadata is clearly indicated.
- [x] Identifier quoting tests pass for supported operations.

### Phase 7 — SQL editor and query execution

**Goal:** Deliver a dependable SQL workflow.

Tasks:

- [x] Integrate CodeMirror 6 or Monaco Editor after evaluating bundle size, licensing, macOS keyboard behaviour, SQL language support, and integration effort.
- [x] Add SQL syntax highlighting and basic editor features: line numbers, indentation, bracket matching, find, and selection.
- [x] Support multiple editor tabs and unsaved-change indicators.
- [x] Allow selecting a connection/database context for each query tab.
- [x] Implement execute selection, or execute the current statement when there is no selection.
- [x] Add a visible Run button and documented keyboard shortcut.
- [x] Execute SQL asynchronously in Rust and keep the UI responsive.
- [x] Add query timeout configuration and cancellation where supported by the driver.
- [x] Return column metadata, rows, affected-row count where available, elapsed duration, and structured errors.
- [x] Support multiple result sets only where the driver can provide them reliably; otherwise report the limitation (first result set shown, extra sets counted).
- [x] Handle statements that return no rows.
- [x] Keep the active connection and target database visible before execution.
- [x] Add an explicit confirmation or strong warning for potentially destructive statements, while recognising that SQL parsing is imperfect and cannot guarantee safety.
- [x] Provide a cancel action and accurate cancelled/running/completed/failed states.
- [x] Avoid automatically retrying write queries after connection loss because their outcome may be uncertain.

**Deliverables:**

- [x] Multi-tab SQL editor.
- [x] Query execution service.
- [x] Query status and error panel.
- [x] Cancellation and timeout handling where supported.

**Exit criteria:**

- [x] Queries work against all three engines.
- [x] Long-running queries do not freeze the UI.
- [x] Errors identify the active connection and provide useful database error information without exposing secrets.
- [x] No automatic unsafe retry occurs.
- [x] Run/cancel states match actual backend state.

### Phase 8 — Results grid, data inspection, and CSV export

**Goal:** Make query results practical for everyday development.

Tasks:

- [x] Build a virtualised results grid that supports large results without rendering every row.
- [x] Show column names, type information where available, row count or returned-row count, and query duration.
- [x] Represent SQL `NULL`, empty strings, binary data, large numeric values, timestamps, and JSON distinctly and safely.
- [x] Avoid silently converting large integers or high-precision decimals into lossy JavaScript numbers. Use string or typed serialisation where required.
- [x] Add horizontal/vertical scrolling, column resizing, column sorting for the rows already fetched, and pagination or bounded fetch controls.
- [x] Make it clear when the displayed rows are only a subset of the total result.
- [x] Add copy-cell, copy-row, and copy-selected-table-as-TSV/CSV actions.
- [x] Add CSV export with correct escaping for commas, quotes, and newlines; use streaming or chunked writing for large exports where practical.
- [x] Let users choose an export destination through a native save dialog.
- [x] Ensure binary and special database types have documented export behaviour.
- [x] Add empty result, query failed, cancelled, and loading states.
- [x] Ensure copying/exporting does not accidentally include hidden columns or unintended rows.

**Deliverables:**

- [x] Efficient results grid.
- [x] Clipboard operations.
- [x] CSV export.

**Exit criteria:**

- [x] Large result sets remain responsive within documented limits.
- [x] Nulls, Unicode, commas, quotes, newlines, and high-precision values are handled correctly.
- [x] Exported CSV opens in common spreadsheet tools and matches the selected output scope.
- [x] The UI distinguishes fetched rows from total rows when total count is unknown.

### Phase 9 — Query history, saved queries, polish, and safeguards

**Goal:** Make the MVP comfortable for daily use.

Tasks:

- [x] Persist query history with timestamp, connection, duration, status, and sanitised error summary.
- [x] Add history search and re-open-in-editor action.
- [x] Add saved queries with names and optional notes.
- [x] Add a clear-history action and history retention setting.
- [x] Add app preferences for theme, editor font size, default result page size, and query timeout where supported.
- [x] Add keyboard shortcuts for new query tab, run query, close tab, focus editor, and refresh explorer.
- [x] Add connection health/status indicators.
- [x] Add confirmation for dangerous actions and clear target-context display.
- [x] Ensure destructive-operation warnings are helpful without becoming a constant nuisance.
- [x] Add unsaved query recovery only if it can be implemented reliably; otherwise warn before closing unsaved tabs.
- [x] Review accessibility, keyboard navigation, focus management, and error wording.
- [x] Polish empty states and onboarding hints.
- [x] Add a privacy note explaining that connections and queries are local, where history is stored, and what is or is not transmitted by the app.

**Deliverables:**

- [x] History and saved-query features.
- [x] Settings.
- [x] Keyboard navigation.
- [x] Security and usability polish.

**Exit criteria:**

- [x] History persists and can be cleared.
- [x] Saved queries can be edited and re-opened.
- [x] Sensitive values are not added to diagnostic logs.
- [x] Important workflows are usable without a mouse.
- [x] Privacy documentation matches actual app behaviour.

### Phase 10 — Testing, packaging, documentation, and MVP release

**Goal:** Produce dependable macOS, Windows and Linux builds suitable for personal use and sharing.

Tasks:

- Run formatting, lint, unit, frontend, Rust, and integration test suites.
- Test with local test instances of MySQL, PostgreSQL, and SQL Server where feasible.
- Test direct remote connections and SSH tunnels.
- Test wrong credentials, invalid ports, network loss, TLS failures, SSH host-key changes, query timeout, cancellation, app restart, and connection cleanup.
- Test database identifiers with spaces, quotes, reserved words, Unicode, and mixed case.
- Test `NULL`, Unicode text, large result sets, binary values, decimal precision, large integers, timestamps, and JSON values.
- Test on macOS (Apple Silicon, then Intel), Windows 10/11 and a current Linux desktop (Ubuntu LTS with GNOME, plus a KDE check for the keyring).
- Verify app data and credential locations; confirm uninstall/documented cleanup behaviour.
- Configure Tauri application identifier, display name, version, bundle icons, permissions/capabilities, and packaging per platform: `.app`/`.dmg` (macOS), MSI/NSIS installer (Windows), AppImage/`.deb` (Linux).
- Audit Tauri capabilities and enabled plugins using least privilege.
- Ensure release builds do not contain development credentials, test endpoints, verbose secret-bearing logs, or debug-only commands.
- Build a release `.app` and a shareable archive/DMG as appropriate.
- Document signing implications per platform: macOS notarisation (Gatekeeper), Windows code signing (SmartScreen), Linux package signing. Do not tell users to bypass security protections blindly.
- Write README installation instructions, supported database versions, known limitations, troubleshooting, data storage locations, and a release checklist.
- Create a known-issues list and versioned changelog.
- Run a clean-install test on each platform (a separate user account or another machine).

**Deliverables:**

- Release build.
- Test report.
- README and troubleshooting guide.
- Known limitations and changelog.
- Repeatable release steps.

**Exit criteria:**

- All release-blocking tests pass.
- No known critical data-loss or credential-exposure issue remains.
- Each database engine's supported capabilities are documented.
- A friend can install and use the build by following the documentation.
- Security and signing limitations on each platform are explained accurately.

### Phase 9b — Inline data editing (done)

Added as its own milestone once connections and queries were proven. See
`docs/decisions/0015-inline-editing.md`: staged cell edits, deletes and
inserts on tables with a primary key, reviewed as SQL and applied in one
transaction; enum and foreign key pickers.

---

## 7b. Extended phases: from MVP to a full-featured client

Phases 11–15 take Thabi Studio from a reliable MVP to a full-featured
daily driver comparable to TablePlus, while keeping its principles: local
first, Rust owns database access, nothing changes on a server without the
user seeing and confirming it, and every engine difference is explicit.

They were chosen by comparing the app (after Phase 9b) with TablePlus
(tableplus.com and docs.tableplus.com, October 2026). Order matters:
each phase builds on the previous one, and Phase 10 (first release) should
ship before Phase 11 starts.

### Read this first (for whoever implements these phases)

These phases may be built by a different engineer or AI agent. Before
writing code:

1. Read `CLAUDE.md`, this plan, and every file in `docs/decisions/`. The
   decision records explain why things are the way they are; don't undo a
   decision without writing a new record that supersedes it.
2. Work on one phase at a time, on its own branch named
   `phase-<n>-<short-name>` (for example `phase-11-connections-safety`).
   The project owner reviews and commits; don't push or merge unless asked.
3. Follow the existing patterns instead of inventing new ones:
   - Rust owns all database access. New SQL that the app generates goes in
     `src-tauri/src/database/` with identifiers quoted by `quote::ident` /
     `quote::table`. Metadata queries bind their values. SQL the user sees
     and owns (filters, staged edits, DDL previews) may use
     `quote::literal`, as decisions 0014 and 0015 explain.
   - Tauri commands stay thin (`src-tauri/src/commands/`); the frontend calls
     them only through `src/lib/tauri/commands.js`.
   - Long work runs on the right session: the connection's main session for
     metadata, a tab's own session (`QueryTab`) for anything the user runs.
     Never put a pool behind an editor.
   - Colours only from `src/styles/tokens.css` (`npm run check:design`
     enforces it). Vue components stay under 250 lines; split by
     responsibility into components and composables.
   - Use the shared primitives (`BaseDialog`, `confirm()`, `useToasts()`,
     `v-tooltip`, `DropdownMenu`, common inputs) and register user actions
     as commands with shortcuts (`useCommandRegistration`), checking
     `docs/design/workspace.md` for reserved keys. Use `Mod+…` so shortcuts
     are ⌘ on macOS and Ctrl on Windows and Linux.
   - Cross-platform: no macOS-only assumptions (see decision 0012).
4. Every change that writes to a server goes through the same review
   pattern as inline editing: build the statements in Rust, show them in a
   review dialog, ask before Production (and before anything destructive),
   run them in a transaction where the engine allows it, and never retry.
5. Each phase adds unit tests and integration tests in `src-tauri/tests/`
   that run against MySQL 8.4, PostgreSQL 16 and SQL Server 2022 via
   `npm run test:integration` (Docker). Run `npm run check` before calling
   anything done.
6. Record each phase's decisions in a new `docs/decisions/00NN-*.md`,
   update `CLAUDE.md` (current status and layout) and tick the tracker in
   §12. Mark a phase complete only when its exit criteria are verified.
7. New dependencies need a reason in the decision record. Prefer the Rust
   side and existing crates; keep the frontend bundle lean.

---

### Phase 11 — Connections, safety modes and workspace

**Goal:** Make connecting and staying safe as flexible as TablePlus,
especially for production servers.

**Already there:** profiles with environment tags, URL import, TLS modes,
SSH (password, key file incl. legacy PEM, agent, keyboard-interactive),
host-key trust, connection rail, switcher (⇧⌘K), Production confirmation
for data-changing statements and staged edits.

Tasks:

- **Safe mode per connection** (replaces the single Production check),
  stored on the profile and shown in the connection pill:
  - *Off*: no prompts.
  - *Confirm writes* (default for Production): confirm statements that can
    change data or schema (`src/lib/sqlSafety.js`), staged edits and DDL.
  - *Confirm everything*: confirm every statement before it runs.
  - *Read-only*: block writes in the app, and on the server where
    possible (`SET SESSION TRANSACTION READ ONLY` on MySQL,
    `default_transaction_read_only = on` on PostgreSQL; SQL Server has no
    session equivalent, so the app check applies and the UI says so).
  - Confirmations name the connection and database, and for Production
    require typing the database name for destructive statements (DROP,
    TRUNCATE, DELETE/UPDATE without WHERE).
- **Connection groups:** user-named folders on the connections page and in
  the switcher; drag to reorder; collapse state remembered. Migration adds
  `group_name` and `sort_order` to `connection_profiles`.
- **Custom colour per connection** in addition to the environment tag
  (picked from the existing `tag-*` tokens only).
- **SSH improvements:**
  - Read `~/.ssh/config` (Host aliases: HostName, User, Port,
    IdentityFile, ProxyJump with one hop). Show what was resolved before
    connecting. Never write to the file.
  - "Ask every time" for SSH passwords and key passphrases (nothing
    stored; a prompt dialog on connect).
  - Optional SSH keep-alive interval.
- **Database connection extras:**
  - "Ask every time" for the database password.
  - Startup commands per connection (for example `SET search_path …`,
    `SET time_zone …`), run on every new session, shown in connection
    details; failures stop the connect with a clear message.
  - TLS client certificate and key (mutual TLS) for MySQL and PostgreSQL;
    SQL Server documents what Tiberius supports.
  - Per-connection query timeout and row limit overriding the app default.
- **Import and export connection profiles** as JSON without secrets (the
  user re-enters passwords). Optionally import a TablePlus `.tableplusconnection`
  export only if its format is documented; otherwise leave it out.
- **Multiple windows:** "New Window" (Mod+Shift+N) opens a second
  workspace window with its own tabs and selected connection. Live
  connections stay shared in Rust; each window owns its query tabs.
- **Reconnect and keep-alive:** detect a dropped session on the next
  action, offer Reconnect in place, and restore the tab's database. Never
  re-run the statement that failed.

Engine notes: read-only and startup commands are per session, so they must
apply to the main session and every query-tab session (`QueryTab::open_session`).

Tests: unit tests for safe-mode classification and SSH config parsing
(include `Include`, wildcards it ignores, quoted values); integration
tests that read-only mode blocks an UPDATE on MySQL and PostgreSQL, that
startup commands run on query-tab sessions, and that a ProxyJump config
reaches the Docker `sshd` through a second bastion container.

**Exit criteria:**

- Each safe mode behaves as described on all three engines, with tests.
- A host alias from `~/.ssh/config` (including one ProxyJump) connects.
- "Ask every time" stores nothing in the credential store (verified).
- Two windows can work on different connections at the same time.
- Connection export/import round-trips everything except secrets.

---

### Phase 12 — Table data and results power-ups

**Goal:** Make browsing and fixing data as fast as TablePlus: find a row,
see it whole, follow relationships, copy it in any shape, and page
through whole tables.

**Already there:** virtualised grid, sorting, value inspector, copy as
TSV/CSV, CSV export, row filters, inline editing with pickers.

Tasks:

- **Real paging for table data:** table tabs load pages with
  `LIMIT/OFFSET` (`OFFSET … FETCH` on SQL Server) ordered by the primary
  key, show "Rows 1–500 of ~12,345" (estimated count from catalog
  statistics, exact count on demand), with previous/next and jump to page.
  Sorting a table tab re-queries the server (`ORDER BY` the column) instead
  of sorting only fetched rows.
- **Row detail panel** (Mod+Alt+I, TablePlus's right panel): the focused
  row as a vertical form of all columns with types; editable through the
  same staged edits; long text and JSON shown in full.
- **Column controls:** hide/show columns (remembered per table), reorder
  by drag, resize to fit, and a "Columns" menu with search.
- **Filter improvements:** an on/off checkbox per filter row, "Apply all"
  (Mod+Return), "Show SQL" for the generated WHERE, quick filter from a
  cell ("Filter by This Value", "Exclude This Value") and from a column
  header, and saved filter presets per table.
- **Foreign key navigation:** a small arrow in FK cells opens the
  referenced row (a table tab filtered to that key); "Show Referencing
  Rows" from a primary key cell lists child tables.
- **Copy and export formats:** copy selection as CSV, TSV, JSON, Markdown
  table, SQL `INSERT` statements and `UPDATE` statements (keyed by the
  primary key); export a result or a whole table to CSV, JSON, SQL
  `INSERT`s, Markdown or Excel `.xlsx` (stream rows from Rust; choose a
  well-maintained crate such as `rust_xlsxwriter` and record it).
- **Edit query results:** when a query's result comes from one table and
  includes its full primary key, allow inline editing exactly like a table
  tab (detect by running a metadata check on the result's columns; else
  stay read-only and say why).
- **More editing conveniences:** duplicate row, paste TSV/CSV rows from
  the clipboard as staged inserts, set a cell to DEFAULT, undo/redo of
  staged edits (Mod+Z inside the grid), and a count of staged changes in
  the tab title.
- **Value display:** JSON pretty viewer with folding, image preview for
  binary data that looks like PNG/JPEG/GIF, date/time shown in a chosen
  time zone (setting), and copy of full binary as hex or base64.

Tests: unit tests for format conversions (CSV quoting, JSON types,
INSERT literal escaping per engine, Markdown escaping) and paging SQL per
engine; integration tests for paging through 10,000 rows with a composite
key, server-side sorting with NULLs, FK navigation, and editing a query
result.

**Exit criteria:**

- A 1-million-row table pages and sorts without loading more than one page
  into the webview.
- Every copy/export format round-trips Unicode, NULL, large numbers and
  binary as documented.
- Editing from a query result is offered only when it's safe, and works on
  all three engines.

---

### Phase 13 — Smart SQL editor

**Goal:** An editor that knows the schema and helps write, format,
explain and reuse SQL.

**Already there:** CodeMirror 6 with dialect highlighting, multiple tabs,
run statement/selection/all, cancel, timeout, history, saved queries in
folders, per-tab database.

Tasks:

- **Schema-aware autocomplete:** databases, schemas, tables, views,
  columns (resolving aliases in the current statement), functions and
  keywords for the active engine; quoted identifiers inserted when needed.
  Metadata comes from a per-connection cache in Rust (loaded lazily,
  refreshed with the explorer) exposed through one command; never block
  typing.
- **Formatter:** Beautify (Mod+I) and Minify for the selection or the
  whole editor, per dialect (evaluate `sql-formatter`, MIT, and record it).
- **Query parameters:** detect `:name`, `$1`/`?` and `@name` placeholders;
  ask for values in a small form (remembered per tab); send them as real
  bind parameters to the driver (add a parameterised execute to
  `DatabaseSession`; map values to text with an optional type per engine).
- **EXPLAIN / query plan:** "Explain" (Mod+Shift+E) runs the engine's plan
  (`EXPLAIN (FORMAT JSON)` and optional `ANALYZE` with a warning on
  PostgreSQL; `EXPLAIN FORMAT=JSON` on MySQL; estimated plan via
  `SET SHOWPLAN_XML ON` on SQL Server) and shows a tree with cost, rows and
  the most expensive nodes highlighted, plus the raw plan.
- **Multiple result sets:** keep every result set a batch returns (today
  only the first is kept) and show each as a result tab.
- **Streaming results:** show the first page while a long query is still
  fetching, with a live row count; cancel keeps what arrived.
- **Split editor panes:** split horizontally or vertically (Mod+Shift+D),
  each pane its own query tab.
- **Snippets with keywords:** saved queries can have a trigger word that
  expands in the editor (TablePlus "keyword binding"); variables like
  `${table}` become tab stops.
- **Run a SQL file:** open a `.sql` file into a tab, or run it directly
  with progress (statement splitting handles MySQL `DELIMITER` and SQL
  Server `GO`), stopping at the first error with its line number.
- **Editor settings:** font family, tab size, word wrap, keyword case,
  vim key bindings as an option.
- **Charts from results:** a simple bar/line chart view of a result
  (choose x and y columns), drawn with an existing small charting library
  or SVG, colours from the tag tokens.

Tests: unit tests for alias resolution, placeholder detection (not
inside strings or comments), statement splitting with DELIMITER/GO and
the formatter wrapper; integration tests for bind parameters with
Unicode, NULL and numeric types on all engines, and for keeping multiple
result sets.

**Exit criteria:**

- Autocomplete suggests correct columns for aliased joins on all engines
  within 100 ms for a 500-table schema (cached).
- Parameters are bound by the driver, never spliced into SQL (verified by
  a test that a value containing quotes and `;` is stored verbatim).
- Plans render for a join on all three engines.

---

### Phase 14 — Schema and object management

**Goal:** Change structure safely from the UI: tables, columns, indexes,
keys, views, routines and triggers, plus a diagram to understand a schema.

**Already there:** explorer of databases, schemas, tables and views;
read-only structure view (columns, indexes, primary key, enum values,
single-column foreign keys).

Tasks:

- **Table designer** (Structure tab becomes editable): add, rename,
  reorder (MySQL only), change type/nullability/default/comment of columns;
  add/drop indexes and unique constraints; set the primary key; add/drop
  foreign keys (multi-column, ON DELETE/UPDATE actions); check constraints.
  Changes are staged and shown as engine-specific DDL in the review dialog
  before running.
- **Create Table** wizard (name, columns, keys, indexes) and **table
  actions** in the explorer: rename, duplicate (structure only or with
  data), truncate, drop, empty; each destructive one confirms by name and
  respects safe mode.
- **DDL generation per engine** in Rust (`database/ddl/`), with known
  differences handled: PostgreSQL DDL is transactional (apply all-or-
  nothing); MySQL commits implicitly per DDL statement (the review dialog
  says so and runs statements one by one, stopping on the first error);
  SQL Server runs in a transaction where allowed.
- **More objects in the explorer:** functions, procedures, triggers,
  sequences, materialized views, user-defined types, with counts; open
  their definition (`pg_get_functiondef`, `SHOW CREATE`, `OBJECT_DEFINITION`)
  read-only in an editor tab; edit and save with `CREATE OR REPLACE` /
  `ALTER` where the engine supports it, through the review dialog.
- **Show CREATE** for any table or view (copy to clipboard or open in a
  tab), and **generate SELECT/INSERT/UPDATE/DELETE templates** from a table.
- **ER diagram (read-only):** for a schema or selected tables, draw tables
  with columns and foreign key lines, auto-laid out (evaluate a small
  layout library such as `elkjs`), zoom/pan, export as SVG/PNG, click to
  open a table.
- **Database and schema actions:** create/drop database (MySQL,
  PostgreSQL, SQL Server) and schema (PostgreSQL, SQL Server), with
  confirmation by name.

Tests: unit tests for DDL generation per engine with awkward identifiers
(spaces, quotes, reserved words, Unicode); integration tests that apply
designer changes and read them back through `describe`, that a failing
PostgreSQL change rolls back, and that routines round-trip their
definition.

**Exit criteria:**

- Every designer change shows its DDL before running and the result
  matches the preview on all three engines.
- No destructive object action runs without naming the object in the
  confirmation.
- The ER diagram renders a 100-table schema smoothly.

---

### Phase 15 — Import, export, backup and administration

**Goal:** Move data in and out, back up and restore, and look after the
server: users, sessions and health.

**Already there:** CSV export of results; Phase 12 adds more formats.

Tasks:

- **Import CSV/TSV/JSON into a table:** preview the first rows, map file
  columns to table columns (or create a new table with inferred types),
  choose delimiter/quote/encoding/header, handle empty values as NULL or
  empty string, insert in batches inside a transaction with progress and
  cancel, report rejected rows with line numbers. Use bind parameters
  (batched multi-row INSERTs; `COPY FROM STDIN` on PostgreSQL if the driver
  supports it).
- **Export database/tables to SQL:** structure, data or both, as a `.sql`
  file the same engine can run; streamed from Rust so memory stays flat.
- **Run SQL dump:** the "Run a SQL file" engine from Phase 13 with an
  import-oriented dialog (target database, stop on error or continue,
  progress).
- **Backup and restore with native tools:** detect `pg_dump`/`pg_restore`,
  `mysqldump`/`mysql` (and `sqlpackage` for SQL Server if present) on the
  PATH or a configured location; run them as child processes with the
  connection's credentials passed through environment variables or a
  temporary option file with owner-only permissions (never on the command
  line), through the SSH tunnel's local port when the connection uses SSH;
  stream progress and logs; cancel cleanly. SQL Server also offers
  server-side `BACKUP DATABASE … TO DISK` / `RESTORE` with a clear note
  that the path is on the server. Restores always confirm by database name.
- **User and permission management:** list users/roles and their grants;
  create/alter/drop users (MySQL users and global/database privileges,
  PostgreSQL roles, membership and GRANTs, SQL Server logins and database
  users with roles); all through the review dialog; passwords entered in a
  password field, never logged or kept.
- **Server activity:** a process list (`pg_stat_activity`,
  `SHOW FULL PROCESSLIST`, `sys.dm_exec_requests`) with refresh, filtering
  and "Kill/Cancel" (confirmed); server info (version, uptime, key
  variables/settings, current connections).
- **Storage overview:** database and table sizes, row estimates and index
  sizes, sortable, with a link to the table.
- **Metrics board (optional, last):** a dashboard tab of tiles, each a
  saved SQL query rendered as a number, table or chart, with manual or
  timed refresh (minimum 10 s) and input fields bound to query parameters.

Tests: integration tests for CSV import with Unicode, NULLs, quotes and a
bad row; SQL export then re-import producing identical data on each
engine; backup/restore round-trip with the native tools in the Docker
images; user creation and grant visible through metadata; killing a
sleeping session.

**Exit criteria:**

- A 1-million-row CSV imports with flat memory and correct row count.
- Backup then restore of the integration database reproduces it on
  MySQL and PostgreSQL, and SQL Server server-side backup works.
- No credential appears in a process list, log, temp file left behind,
  or error message (tested).

---

### Still out of scope after Phase 15

- User accounts, cloud sync, team workspaces, hosted services, licensing.
- Telemetry of any kind.
- AI query generation (may be revisited as an opt-in, local-first
  feature with its own decision record).
- Plugin system or marketplace.
- New engines (SQLite, MariaDB-specific features, CockroachDB, Redis,
  MongoDB). SQLite is the most likely next engine; it would need its own
  phase with a `DatabaseSession` implementation and tests.
- Mobile apps.
- Automatic updates until signed releases exist on all three platforms.

---

## 8. Cross-cutting security checklist

Apply throughout every phase:

- [ ] Database credentials and SSH secrets use OS secure storage where supported.
- [ ] No plaintext secrets in SQLite, config files, source code, logs, screenshots, crash reports, or error messages.
- [ ] TLS certificate verification is enabled by default.
- [ ] SSH host keys are verified; changed keys require user attention.
- [ ] Tauri capabilities and plugin permissions follow least privilege.
- [ ] Only necessary Tauri commands are exposed to the frontend.
- [ ] Validate all frontend-provided IDs, paths, settings, and request parameters in Rust.
- [ ] SQL generated by the app uses parameterised values and database-specific identifier quoting.
- [ ] User-authored SQL is clearly executed against the selected connection/database.
- [ ] Dangerous query warnings are treated as a usability safeguard, not a security boundary.
- [ ] Query timeouts and cancellation are implemented where supported.
- [ ] Write operations are not automatically retried after ambiguous network failures.
- [ ] Export paths are selected safely and file-writing errors are shown clearly.
- [ ] Local query history can be disabled, retained for a limited time, or cleared.
- [ ] Test credentials are fake and stored outside the repository.
- [ ] Release builds are reviewed for debug commands and verbose logging.
- [ ] Third-party crate and frontend dependency licences are reviewed before sharing binaries.
- [ ] Dependencies are updated deliberately and builds are retested after upgrades.

---

## 9. Testing strategy

### Unit tests

Test:

- Connection form validation.
- Driver selection and capability mapping.
- Connection state transitions.
- Secret reference lifecycle.
- Error sanitisation.
- SQL identifier quoting.
- CSV escaping and encoding.
- Query-history retention.
- Settings validation.
- Result serialisation for `NULL`, decimal, big integer, timestamp, JSON, and binary values.

### Integration tests

Use dedicated local test databases and environment-provided credentials. Never embed secrets in tests or commit `.env` files containing real credentials.

For each engine, cover:

- Connect/disconnect.
- Simple `SELECT`.
- Empty result.
- Multiple columns and rows.
- Unicode values.
- Null values.
- Large integer and decimal values.
- Query error.
- Table/schema listing.
- Timeout/cancellation where supported.
- Connection interruption and cleanup.

For SSH:

- Successful tunnel and query.
- Incorrect SSH credentials.
- Untrusted or changed host key.
- Unreachable remote database.
- Database connection failure after tunnel setup.
- Tunnel shutdown after disconnect.

### Manual release checklist

- [ ] Launch from a clean install.
- [ ] Create, edit, test, and delete each connection type.
- [ ] Restart and verify saved profiles remain.
- [ ] Verify secrets remain in secure storage and are not exposed in SQLite.
- [ ] Connect to local and remote databases.
- [ ] Connect through SSH to a private database.
- [ ] Browse schemas and table metadata.
- [ ] Run successful, empty, failing, long-running, and cancelled queries.
- [ ] Export CSV and verify special characters.
- [ ] Close tabs and the application with active operations.
- [ ] Check logs for secrets.
- [ ] Confirm documentation reflects the release.

---

## 10. Claude Code workflow

Use Claude Code Projects to maintain project context and implement one phase at a time. Keep this plan and project rules in the repository so that they remain available across sessions.

### `CLAUDE.md` requirements

Create and maintain a concise `CLAUDE.md` containing:

- Product purpose and MVP scope.
- Stack and architecture.
- Current phase and completed milestones.
- Commands to install, run, lint, test, and build.
- Coding standards and file-size limits.
- Security requirements.
- Database driver decisions and known limitations.
- Rules for updating `plan.md` after completing work.

### Coding rules

1. Use JavaScript for Vue unless explicitly approved otherwise.
2. Keep Vue components below 250 lines. If a component approaches the limit, split by meaningful responsibility.
3. Use reusable components for repeated form controls, dialogs, notifications, and layout patterns.
4. Keep business logic out of large Vue components; use composables, stores, and Rust services appropriately.
5. Use only design tokens/theme colours defined in the central stylesheet or theme system.
6. Do not duplicate database-specific logic across UI components.
7. Do not make database connections from the frontend.
8. Never add real credentials or secrets to source control.
9. Do not invent success states, fake data, or placeholder implementations that look production-ready.
10. Add tests for each important feature and regression.
11. Run the relevant checks after every meaningful change.
12. Keep dependencies minimal; explain why each new dependency is needed.
13. Review Tauri permissions and capabilities whenever adding a plugin or native capability.
14. Update docs and this plan when architecture or scope changes.
15. Do not refactor unrelated code during a phase unless necessary.

### Recommended phase execution loop

For each phase, ask Claude Code to:

1. Inspect the current repository and relevant files.
2. Summarise the existing state and identify the exact work needed for the phase.
3. Propose a small implementation plan with files to add/change.
4. Implement a coherent subset rather than generating the entire phase in one pass.
5. Add or update tests.
6. Run formatting, lint, build, and relevant tests.
7. Fix failures caused by the change.
8. Review for security, accessibility, error handling, and regressions.
9. Update the phase checklist in `plan.md`.
10. Summarise changed files, commands run, results, remaining risks, and the next task.

Do not mark a phase complete because code was written. Mark it complete only when the exit criteria have been verified or any exception is clearly documented.

### Initial instruction to Claude Code

> Read `plan.md` and `CLAUDE.md` before making changes. Inspect the existing repository and environment. Do not implement the whole application at once. Begin with Phase 1 only: report the current state, propose the smallest safe changes, implement the scaffold and baseline, run the documented checks, and update the Phase 1 checklist. Do not begin Phase 2 until Phase 1 exit criteria are met.

---

## 11. Definition of done

### MVP (v0.x)

The MVP is complete when all of the following are true:

- [ ] Tauri desktop app runs and builds on macOS, Windows and Linux.
- [ ] App uses Vue 3 and JavaScript for the frontend and Rust for native/database work.
- [ ] MySQL, PostgreSQL, and SQL Server each have a verified driver implementation.
- [ ] Saved connection profiles persist locally.
- [ ] Credentials are stored securely and excluded from logs.
- [ ] Direct remote connections and SSH tunnelling work for documented supported configurations.
- [ ] TLS and SSH host-key validation behave securely.
- [ ] Database/schema/table explorer works.
- [ ] Multi-tab SQL editor executes queries against the selected connection.
- [ ] Results grid handles pagination/bounded retrieval and common database value types safely.
- [ ] Query history and saved queries work and can be cleared.
- [ ] CSV export and clipboard copy work correctly.
- [ ] Timeouts/cancellation are supported where drivers permit them and limitations are explicit.
- [ ] Error, empty, loading, and disconnected states are implemented.
- [ ] Unit and integration tests cover the critical paths.
- [ ] README, setup instructions, known limitations, and release notes are complete.
- [ ] Release build has been tested from a clean install.
- [ ] No unresolved critical security, credential exposure, or data-loss defects remain.

---

### Full-featured (v1.0)

Version 1.0 is complete when, in addition to the MVP:

- [ ] Phases 11–15 meet their exit criteria, or each exception is documented.
- [ ] Every feature that writes to a server previews its SQL, respects the connection's safe mode, and is integration-tested on MySQL, PostgreSQL and SQL Server.
- [ ] Memory stays bounded for 1-million-row tables, imports and exports.
- [ ] No credential appears in logs, temp files, process arguments or errors.
- [ ] Signed builds exist for macOS, Windows and Linux, with documented install steps.
- [ ] README, per-engine capability matrix and changelog cover the new features.

---

## 12. Progress tracker

Update this section as work progresses.

- [x] Phase 1 — Project scaffold and engineering baseline (2026-10-14)
  - [x] Tauri 2 + Vue 3 (JavaScript) + Tailwind v4 scaffold; `npm run tauri dev` launches on macOS
  - [x] `npm run check` passes: Prettier, ESLint, Vite build, clippy (`-D warnings`), `cargo test`
  - [x] Clean copy of the repo installs with `npm ci` and passes `npm run check`
  - [x] App shell, `ErrorBoundary`, toast and confirm/dialog primitives
  - [x] Design tokens with light/dark themes following macOS appearance
  - [x] `CLAUDE.md`, `README.md`, `docs/decisions/0001-phase-1-baseline.md`
  - [x] Template demo command and unused opener plugin removed; strict CSP set
  - Note: CSP is verified against a release build in Phase 10
- [x] Phase 2 — Product shell and design system (2026-10-14)
  - [x] Workspace design agreed: `docs/design/workspace.md` (layout, principles, copy, shortcuts)
  - [x] Semantic colour tokens for light and dark; Tailwind palette disabled; contrast and hard-coded colour check in `npm run check`
  - [x] Unified title bar, resizable sidebar (Explorer/Saved/History), status bar, first-connection empty state
  - [x] Components: buttons, icon buttons, text input, select, checkbox, switch, segmented control/tabs, dropdown menu, tooltip, dialog, confirm, toast, spinner, empty and error states
  - [x] Command registry with macOS shortcuts, Open Anything (⌘P), Settings (⌘,) with appearance, Keyboard Shortcuts (⌘/)
  - [x] Keyboard: visible focus, labelled controls, arrow-key menus and segmented controls, keyboard-resizable sidebar
  - [x] Narrow windows hide the sidebar below 1000px; ⌘1 toggles it
  - [x] All components under 250 lines; Vitest covers shortcut and fuzzy logic
  - Note: theme and layout preferences initially used localStorage; Phase 3 migrated them into SQLite.
- [x] Phase 3 — Local persistence and connection profiles (2026-10-14)
  - [x] Versioned SQLite migrations persist profiles and preferences; legacy theme/layout preferences migrate once
  - [x] Create, edit, duplicate, delete, list and select database-specific connection profiles
  - [x] Store database and SSH passwords in the OS credential store with opaque SQLite references
  - [x] Field-specific validation, password visibility controls and explicit disabled test-connection state
  - [x] CRUD, restart-persistence, secure-secret-boundary and forward-migration tests
  - Note: no live database connections exist yet; profile deletion will close one after Phase 4 adds connection lifecycle
- [x] Phase 4 — Database driver proof of concept and connection lifecycle (2026-10-15)
  - [x] SQLx 0.9 (MySQL, PostgreSQL) and Tiberius 0.13 (SQL Server); one session per connection
  - [x] Driver contract, per-connection TLS modes, specific sanitised errors, lifecycle states
  - [x] 23 integration tests pass against PostgreSQL 16, MySQL 8.4 and SQL Server 2022 on Apple Silicon
  - [x] Review fixes applied: `docs/reviews/2026-10-15-phase-3-4-review.md`, `docs/decisions/0006-connection-model.md`
  - Open: verify against a remote server per engine; query cancellation moves to Phase 7 (capabilities report it as unsupported)
- [x] Phase 5 — Remote connectivity, TLS, and SSH tunnelling (2026-10-15, branch `phase-5-remote-ssh`)
  - [x] SSH tunnels with russh: password, key file (+ passphrase) and agent sign-in
  - [x] Host keys checked against Thabi's trusted keys and ~/.ssh/known_hosts; unknown keys need confirmation, changed keys are refused
  - [x] Tunnel closes with its session; 5 SSH integration tests pass against a Docker bastion
  - [x] Status bar security indicator
  - Open: check a real remote server directly and through SSH
- [x] Phase 6 — Database explorer and metadata (2026-10-15, branch `phase-6-explorer`)
  - [x] Lazy, keyboard-accessible tree for all three engines; PostgreSQL switch-database flow
  - [x] Structure view (columns, keys, indexes) and read-only 100-row data preview
  - [x] Identifier quoting with tests; permission-denied, lost-connection and empty states
  - [x] 31 integration tests pass, including awkward names on every engine
- [x] Phase 7 — SQL editor and query execution (2026-10-15, branch `phase-7-sql-editor`, not committed)
  - [x] CodeMirror 6 editor in workspace tabs; run statement/selection/all; database per tab
  - [x] One session per query tab; cancel (server-side for PostgreSQL/MySQL, session reset for SQL Server); timeout preference; no retries
  - [x] Confirmation for risky statements and for writes on Production
  - [x] Native menu bar; 37 integration tests pass
- [x] Phase 8 — Results grid, data inspection, and CSV export (2026-10-15, branch `phase-8-results-grid`, not committed)
  - [x] Results kept in Rust up to a configurable limit; virtualised grid fetches pages
  - [x] Sort, resize, select, inspect values (JSON, binary, NULL, empty)
  - [x] Copy as TSV/CSV and streamed CSV export with the native save dialog
  - [x] Explorer and query tabs scoped to a connection's saved database; USE follows
  - [x] 39 integration tests pass
- [x] Extra (2026-10-15, branch `connections-and-table-data`, not committed): table data in the full grid with export; connections page grouped by environment with search, richer rows, recent connections, quick switcher (⇧⌘K) and connection URL paste (`docs/decisions/0011`)
- [x] Phase 9 — Query history, saved queries, polish, and safeguards (2026-10-15, branch `phase-9-history-saved-queries`)
  - [x] Cross-platform (macOS, Windows, Linux): credential stores, window chrome, menus, SSH agent, wording, CI for all three
  - [x] Query history (recorded per run, searchable, retention, clear) and saved queries (⌘S)
  - [x] Open query tabs restored after quitting; editor font size and history preferences
  - [x] Privacy note in About and `docs/PRIVACY.md`; accessibility pass
  - [x] Saved-query folders; SSH keyboard-interactive and legacy PEM keys; connection rail; table row filters
- [x] Phase 9b — Inline data editing, added as its own milestone (2026-10-15, branch `phase-9b-inline-editing`, merged into `main`)
  - [x] Edit cells, Set to NULL, delete rows and add rows on tables with a primary key; staged until applied
  - [x] Review the exact SQL; apply all changes in one transaction; each must change exactly one row or nothing is saved
  - [x] Asks before deleting rows and before any change on Production; integration-tested on all three engines
- [ ] Phase 10 — Testing, packaging, documentation, and MVP release (deferred until signing options exist; only a Mac is available for clean-install tests)
- [ ] Phase 11 — Connections, safety modes and workspace (branch `phase-11-connections-safety`)
  - [x] Safe mode per connection: off, confirm writes, confirm everything, read-only (`docs/decisions/0016-safe-modes.md`)
  - [x] Replaces the Production-only check: destructive statements still ask, and Production types the database name
  - [x] Read-only refused in the app and on the server (`SET SESSION TRANSACTION READ ONLY`, `default_transaction_read_only`); SQL Server documented as app-only
  - [x] Staged edits follow the mode; a read-only connection can't be edited
  - [ ] Connection groups and custom colours
  - [ ] `~/.ssh/config`, "ask every time" secrets, SSH keep-alive
  - [ ] Startup commands, mutual TLS, per-connection timeout and row limit
  - [ ] Import/export without secrets, multiple windows, reconnect
- [ ] Phase 12 — Table data and results power-ups
- [ ] Phase 13 — Smart SQL editor
- [ ] Phase 14 — Schema and object management
- [ ] Phase 15 — Import, export, backup and administration

**Final principle:** Build the smallest genuinely reliable database client first. A polished connection flow, accurate query results, safe credential handling, and dependable remote access are more valuable than many incomplete features.

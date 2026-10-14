# Thabi Studio — MVP Development Plan

**Product:** Thabi Studio  
**Product type:** Native desktop database client  
**Initial platforms:** macOS first; keep Windows/Linux portability in mind  
**Stack:** Tauri 2, Vue 3, JavaScript, Rust  
**Initial database engines:** MySQL, PostgreSQL, Microsoft SQL Server  
**Primary use:** Personal daily development work, with builds shared with developer friends  
**Document purpose:** Phased implementation plan for Claude Code Projects

---

## 1. Product vision

Thabi Studio is a polished, local-first desktop application for connecting to, exploring, querying, and managing MySQL, PostgreSQL, and Microsoft SQL Server databases from one workspace.

It should feel like a carefully designed native Mac developer tool: fast, calm, keyboard-friendly, and reliable. The app connects directly from the user's computer to database servers. **No Thabi-hosted backend, user account, telemetry service, or cloud dependency is required for the MVP.**

The MVP should prioritise reliable connections, safe query execution, useful schema exploration, and a high-quality results grid over a long list of advanced features.

### MVP success criteria

A user can:

1. Install and launch Thabi Studio on macOS.
2. Create, edit, duplicate, test, and delete saved connection profiles.
3. Connect to local or remote MySQL, PostgreSQL, and Microsoft SQL Server databases.
4. Connect directly over TCP/TLS or through an SSH tunnel.
5. Browse available databases, schemas, tables, views, columns, and indexes where supported.
6. Open multiple SQL editor tabs, execute queries, cancel queries where the driver supports cancellation, and view results and errors.
7. Inspect large result sets using pagination or bounded result retrieval rather than loading unlimited rows into memory.
8. Copy selected cells/rows and export query results to CSV.
9. View query history and save reusable SQL snippets/queries.
10. Close connections and SSH tunnels cleanly, with credentials stored securely.

---

## 2. Scope boundaries

### Included in MVP

- macOS-first desktop packaging with Tauri 2.
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

### Explicitly out of scope for MVP

Do not start these unless all MVP acceptance criteria are met:

- User accounts, cloud sync, team workspaces, hosted services, or a licensing server.
- AI query generation or AI assistant features.
- Visual query builder.
- Full ERD/schema diagramming.
- Cross-database schema/data comparison and migration tooling.
- Database backup/restore orchestration.
- Import wizard for many file formats.
- Table designer capable of changing arbitrary schema objects.
- Advanced data editing with bulk insert/update/delete.
- Stored procedure/function editors and debugger.
- Replication, monitoring dashboards, performance profiling, or explain-plan visualisation.
- Plugin marketplace.
- Automatic updates unless a safe and practical release mechanism is implemented after the core app is stable.

A basic read-only data browsing view may be included. Do not promise full inline data editing in the MVP unless it is added as a separately scoped milestone after connection/query reliability is proven.

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
- Check crate versions, licensing, release activity, supported Rust toolchain, macOS Apple Silicon builds, and whether the chosen driver can use a TCP stream supplied by the SSH tunnel implementation.
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
- Confirm `cargo check`, frontend build, and Tauri development launch work on the Mac.
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
- The app opens on macOS without console or Rust errors.
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
- Add keyboard shortcuts infrastructure without binding shortcuts that conflict with macOS conventions.

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

- Add SQLite migrations for connection profile metadata and app settings.
- Add create, edit, duplicate, delete, list, and select profile operations.
- Build a connection form that adapts to MySQL, PostgreSQL, and SQL Server defaults without assuming identical options.
- Validate required fields, ports, hostnames, database names, and supported authentication options.
- Provide show/hide password controls and safe password-field behaviour.
- Integrate OS-backed secure storage for passwords and supported SSH secrets.
- Store opaque secret references rather than secret values in SQLite.
- Handle credential update, removal, missing-secret, and secure-store access failures.
- Add connection labels, optional colour/icon, and recent connection timestamp if useful.
- Ensure deleting a profile can close its active connection and remove its associated stored credentials after confirmation.
- Implement a “Test connection” UI state, but do not report success until a real driver test exists in later phases.

**Deliverables:**

- Persistent connection manager.
- Database-specific connection forms.
- Secure credential-store abstraction.
- SQLite migrations and repository tests.

**Exit criteria:**

- Profiles survive app restart.
- Secrets are not present in SQLite records or logs.
- Invalid forms show clear, field-specific errors.
- CRUD and migration tests pass.
- No fake connection-success state is presented.

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
- Add connection testing through the real selected driver.
- Verify connections against local development databases and at least one remote environment per supported engine where available.
- Document required environment variables or test configuration without committing credentials.

**Deliverables:**

- Three functioning driver adapters.
- Connection lifecycle service.
- Real connection test feature.
- Driver feasibility notes and integration tests.

**Exit criteria:**

- Each engine can establish a real connection, run a simple query, and close cleanly.
- Failures such as wrong password, unreachable host, refused port, timeout, and invalid TLS configuration produce useful errors without leaking secrets.
- macOS Apple Silicon build works.
- SQL Server limitations, if any, are documented explicitly.

### Phase 5 — Remote connectivity, TLS, and SSH tunnelling

**Goal:** Support secure local and remote database access.

Tasks:

- Implement direct TCP connectivity for each supported driver.
- Configure TLS and certificate verification with secure defaults, subject to driver capabilities.
- Support explicit trust configuration where appropriate; clearly warn about disabling certificate verification.
- Implement SSH local port forwarding with a maintained Rust SSH library after checking platform compatibility.
- Support SSH host, port, username, and selected authentication methods.
- Verify SSH host keys; do not silently accept unknown or changed host keys.
- Support SSH key selection/passphrase handling only through a safe, documented design.
- Map the local forwarded endpoint to the selected database connection without exposing the remote database publicly.
- Tie tunnel lifetime to the associated database connection; close the tunnel on disconnect, profile deletion, app shutdown, and failed connection setup.
- Handle tunnel conflicts, SSH authentication errors, host-key mismatch, network loss, and remote endpoint failure.
- Add test coverage for connection and tunnel cleanup.
- Document that the user's existing VPN/private network can be used where applicable.

**Deliverables:**

- Direct remote connections.
- TLS configuration.
- SSH tunnel management.
- Clear connection security indicators.

**Exit criteria:**

- A user can connect to a remote database directly when network access permits.
- A user can connect to a private database through an SSH tunnel.
- TLS and SSH host-key failures are surfaced rather than bypassed.
- No tunnel is left running after the owning connection is closed.
- Secrets and private key material never appear in logs.

### Phase 6 — Database explorer and metadata

**Goal:** Browse database structure reliably.

Tasks:

- Build a lazy-loaded tree for connections, databases, schemas, tables, and views.
- Load child nodes on demand rather than querying every object at once.
- Implement driver-specific metadata queries.
- Display columns, types, nullability, primary keys, indexes, and other metadata where supported.
- Distinguish unsupported features from empty results.
- Add refresh for the selected node and refresh-all where reasonable.
- Add object context menus for supported actions, starting with “Open data” and “Copy name”.
- Quote/escape identifiers using database-specific rules for any generated SQL.
- Display loading, permission-denied, stale-connection, and empty-schema states.
- Avoid blocking the UI while metadata is fetched.
- Add metadata tests for names containing spaces, reserved words, mixed case, and non-ASCII characters.

**Deliverables:**

- Schema explorer.
- Table/view structure panel.
- Metadata adapters for each database.

**Exit criteria:**

- Users can navigate database objects for all three engines.
- Explorer requests are lazy and cancellable where practical.
- Unsupported metadata is clearly indicated.
- Identifier quoting tests pass for supported operations.

### Phase 7 — SQL editor and query execution

**Goal:** Deliver a dependable SQL workflow.

Tasks:

- Integrate CodeMirror 6 or Monaco Editor after evaluating bundle size, licensing, macOS keyboard behaviour, SQL language support, and integration effort.
- Add SQL syntax highlighting and basic editor features: line numbers, indentation, bracket matching, find, and selection.
- Support multiple editor tabs and unsaved-change indicators.
- Allow selecting a connection/database context for each query tab.
- Implement execute selection, or execute the current statement when there is no selection.
- Add a visible Run button and documented keyboard shortcut.
- Execute SQL asynchronously in Rust and keep the UI responsive.
- Add query timeout configuration and cancellation where supported by the driver.
- Return column metadata, rows, affected-row count where available, elapsed duration, and structured errors.
- Support multiple result sets only where the driver can provide them reliably; otherwise report the limitation.
- Handle statements that return no rows.
- Keep the active connection and target database visible before execution.
- Add an explicit confirmation or strong warning for potentially destructive statements, while recognising that SQL parsing is imperfect and cannot guarantee safety.
- Provide a cancel action and accurate cancelled/running/completed/failed states.
- Avoid automatically retrying write queries after connection loss because their outcome may be uncertain.

**Deliverables:**

- Multi-tab SQL editor.
- Query execution service.
- Query status and error panel.
- Cancellation and timeout handling where supported.

**Exit criteria:**

- Queries work against all three engines.
- Long-running queries do not freeze the UI.
- Errors identify the active connection and provide useful database error information without exposing secrets.
- No automatic unsafe retry occurs.
- Run/cancel states match actual backend state.

### Phase 8 — Results grid, data inspection, and CSV export

**Goal:** Make query results practical for everyday development.

Tasks:

- Build a virtualised results grid that supports large results without rendering every row.
- Show column names, type information where available, row count or returned-row count, and query duration.
- Represent SQL `NULL`, empty strings, binary data, large numeric values, timestamps, and JSON distinctly and safely.
- Avoid silently converting large integers or high-precision decimals into lossy JavaScript numbers. Use string or typed serialisation where required.
- Add horizontal/vertical scrolling, column resizing, column sorting for the rows already fetched, and pagination or bounded fetch controls.
- Make it clear when the displayed rows are only a subset of the total result.
- Add copy-cell, copy-row, and copy-selected-table-as-TSV/CSV actions.
- Add CSV export with correct escaping for commas, quotes, and newlines; use streaming or chunked writing for large exports where practical.
- Let users choose an export destination through a native save dialog.
- Ensure binary and special database types have documented export behaviour.
- Add empty result, query failed, cancelled, and loading states.
- Ensure copying/exporting does not accidentally include hidden columns or unintended rows.

**Deliverables:**

- Efficient results grid.
- Clipboard operations.
- CSV export.

**Exit criteria:**

- Large result sets remain responsive within documented limits.
- Nulls, Unicode, commas, quotes, newlines, and high-precision values are handled correctly.
- Exported CSV opens in common spreadsheet tools and matches the selected output scope.
- The UI distinguishes fetched rows from total rows when total count is unknown.

### Phase 9 — Query history, saved queries, polish, and safeguards

**Goal:** Make the MVP comfortable for daily use.

Tasks:

- Persist query history with timestamp, connection, duration, status, and sanitised error summary.
- Add history search and re-open-in-editor action.
- Add saved queries with names and optional notes.
- Add a clear-history action and history retention setting.
- Add app preferences for theme, editor font size, default result page size, and query timeout where supported.
- Add keyboard shortcuts for new query tab, run query, close tab, focus editor, and refresh explorer.
- Add connection health/status indicators.
- Add confirmation for dangerous actions and clear target-context display.
- Ensure destructive-operation warnings are helpful without becoming a constant nuisance.
- Add unsaved query recovery only if it can be implemented reliably; otherwise warn before closing unsaved tabs.
- Review accessibility, keyboard navigation, focus management, and error wording.
- Polish empty states and onboarding hints.
- Add a privacy note explaining that connections and queries are local, where history is stored, and what is or is not transmitted by the app.

**Deliverables:**

- History and saved-query features.
- Settings.
- Keyboard navigation.
- Security and usability polish.

**Exit criteria:**

- History persists and can be cleared.
- Saved queries can be edited and re-opened.
- Sensitive values are not added to diagnostic logs.
- Important workflows are usable without a mouse.
- Privacy documentation matches actual app behaviour.

### Phase 10 — Testing, packaging, documentation, and MVP release

**Goal:** Produce a dependable macOS build suitable for personal use and sharing.

Tasks:

- Run formatting, lint, unit, frontend, Rust, and integration test suites.
- Test with local test instances of MySQL, PostgreSQL, and SQL Server where feasible.
- Test direct remote connections and SSH tunnels.
- Test wrong credentials, invalid ports, network loss, TLS failures, SSH host-key changes, query timeout, cancellation, app restart, and connection cleanup.
- Test database identifiers with spaces, quotes, reserved words, Unicode, and mixed case.
- Test `NULL`, Unicode text, large result sets, binary values, decimal precision, large integers, timestamps, and JSON values.
- Test macOS Apple Silicon first; test Intel only if the target build environment and dependencies support it.
- Verify app data and credential locations; confirm uninstall/documented cleanup behaviour.
- Configure Tauri application identifier, display name, version, bundle icons, permissions/capabilities, and macOS packaging.
- Audit Tauri capabilities and enabled plugins using least privilege.
- Ensure release builds do not contain development credentials, test endpoints, verbose secret-bearing logs, or debug-only commands.
- Build a release `.app` and a shareable archive/DMG as appropriate.
- Document signing/notarisation implications. Unsigned builds may trigger macOS Gatekeeper warnings; do not tell users to bypass security protections blindly.
- Write README installation instructions, supported database versions, known limitations, troubleshooting, data storage locations, and a release checklist.
- Create a known-issues list and versioned changelog.
- Run a clean-install test on a separate macOS user account or another Mac if available.

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
- macOS security/signing limitations are explained accurately.

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

## 11. MVP definition of done

The MVP is complete when all of the following are true:

- [ ] Tauri desktop app runs and builds on macOS.
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

## 12. Progress tracker

Update this section as work progresses.

- [ ] Phase 1 — Project scaffold and engineering baseline
- [ ] Phase 2 — Product shell and design system
- [ ] Phase 3 — Local persistence and connection profiles
- [ ] Phase 4 — Database driver proof of concept and connection lifecycle
- [ ] Phase 5 — Remote connectivity, TLS, and SSH tunnelling
- [ ] Phase 6 — Database explorer and metadata
- [ ] Phase 7 — SQL editor and query execution
- [ ] Phase 8 — Results grid, data inspection, and CSV export
- [ ] Phase 9 — Query history, saved queries, polish, and safeguards
- [ ] Phase 10 — Testing, packaging, documentation, and MVP release

**Final principle:** Build the smallest genuinely reliable database client first. A polished connection flow, accurate query results, safe credential handling, and dependable remote access are more valuable than many incomplete features.

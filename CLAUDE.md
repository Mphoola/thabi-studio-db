# Thabi Studio — notes for Claude

Read `plan.md` before making changes. It is the source of truth for scope,
phases and exit criteria. Work one phase at a time and do not start the next
phase until the current one's exit criteria are verified.

## Product

Local-first desktop client for MySQL, PostgreSQL and SQL Server on macOS,
Windows and Linux (developed on macOS; CI builds all three).
No Thabi backend, accounts, telemetry or cloud dependency. The MVP favours
reliable connections, safe query execution, schema browsing and a good
results grid over breadth of features. See `plan.md` §2 for what is out of scope.

## Current status

- Phase 1 (scaffold and engineering baseline): complete.
- Phase 2 (product shell and design system): complete. Layout, principles,
  copy and shortcuts live in `docs/design/workspace.md`; follow them.
- Phase 3 (local persistence and connection profiles): complete. SQLite profile
  and preference storage is in place; passwords use Keychain.
- Phase 4 (database drivers and connection lifecycle): complete. Read
  `docs/decisions/0006-connection-model.md` before touching connections.
  Remote-server verification is still open (see plan.md).
- Phase 5 (remote connectivity and SSH tunnels): complete on branch
  `phase-5-remote-ssh`. Read `docs/decisions/0007-ssh-tunnels.md`.
- Phase 6 (database explorer): complete on branch `phase-6-explorer`. Read
  `docs/decisions/0008-explorer.md`.
- Phase 7 (SQL editor): committed on `phase-7-sql-editor`. Read
  `docs/decisions/0009-sql-editor.md`.
- Phase 8 (results grid): committed on `phase-8-results-grid`. Read
  `docs/decisions/0010-results-grid.md`.
- Connections page and table data improvements: committed on
  `connections-and-table-data` (`docs/decisions/0011`).
- Cross-platform support and Phase 9 (history, saved queries, recovery):
  on `phase-9-history-saved-queries`, not committed (the user tests first).
  Read `docs/decisions/0012-cross-platform.md` and `0013-history-saved-queries.md`.
  Also on this branch: saved-query folders, SSH fixes (keyboard-interactive,
  legacy PEM keys), the connection rail and table row filters (`0014`).
- Phase 9b (inline data editing): on `phase-9b-inline-editing`, not
  committed. Read `docs/decisions/0015-inline-editing.md`.
- Next: Phase 10 (testing, packaging for all three platforms, release).

Update this section and the tracker in `plan.md` §12 when a phase finishes.

## Stack

- Tauri 2 (Rust) — owns all database access, credentials, SSH and file I/O.
- Vue 3 + Vite, **JavaScript only** (no TypeScript unless explicitly approved).
- Tailwind CSS v4 via `@tailwindcss/vite`.
- Decisions and versions: `docs/decisions/`.

## Commands

```bash
npm install            # install frontend deps (Rust deps build on first run)
npm run tauri dev      # run the desktop app with hot reload
npm run check          # format, ESLint, design check, Vitest, build, clippy, cargo test
npm run test           # Vitest unit tests
npm run format         # Prettier + rustfmt
npm run lint:fix       # ESLint autofix
npm run test:integration  # driver tests against MySQL/Postgres/SQL Server in Docker
npm run tauri build    # release .app/.dmg (slow: LTO)
```

Run `npm run check` after every meaningful change and before calling work done.

## Layout

- `src/components/app/` — app shell (sidebar, status bar, error boundary, About).
- `src/components/app/` also holds the title bar, sidebar, command palette,
  settings and shortcuts dialogs.
- `src/components/common/` — reusable primitives: `BaseButton`, `IconButton`,
  `TextInput`, `SelectInput`, `CheckboxInput`, `SwitchInput`,
  `SegmentedControl`, `DropdownMenu`, `BaseDialog`, `EmptyState`,
  `InlineAlert`, `SpinnerIcon`, `KbdShortcut`, plus the toast, confirm and
  tooltip hosts. See them all in the dev-only Component Gallery (⌘P).
- `src/components/dev/` — development-only screens, never in release builds.
- `src/composables/` — shared state: `useCommands` (commands + shortcuts),
  `useAppCommands`, `useLayout`, `useDialogs`, `useTheme`, `useToasts`,
  `useConfirm`, `useTooltip` (`v-tooltip`), `useAppInfo`.
- `src/lib/` — pure helpers (`shortcuts.js`, `fuzzy.js`, `position.js`),
  unit-tested in `tests/frontend/`.
- `src/lib/tauri/` — the only place that calls `invoke`. Add new command
  wrappers to `commands.js`; errors arrive as `{ kind, message }`.
- `src/styles/tokens.css` — the only place colours, type, radii and shadows are defined.
- `src-tauri/src/commands/` — thin async Tauri commands, one module per area.
  SQLite/Keychain work goes through `AppState::blocking`.
- `src-tauri/src/database/` — driver contract (`DatabaseSession`), one
  session type per engine, result values and error mapping (`error.rs`).
- `src-tauri/src/connections.rs` — live connections and their states.
- `src-tauri/src/profiles/` — profile validation and storage.
- `src-tauri/src/ssh/` — SSH tunnels and trusted host keys.
- `src-tauri/src/database/quote.rs` — identifier quoting; use it for every
  generated identifier. Values always go through bind parameters.
- `src/components/explorer/` — explorer tree, object, structure and data views.
- `src/components/workspace/` — tab bar, query tab, SQL editor (CodeMirror), results.
- `src-tauri/src/queries.rs` — query-tab sessions, cancel and timeout.
- `src-tauri/src/menu.rs` — native menu bar; items forward command ids.
- `src-tauri/src/results.rs` — kept results: paging, sorting, copy, CSV export.
- `src-tauri/src/library.rs` — query history, saved queries, open-tab recovery.
- `src/components/library/` — History and Saved panels, Save Query dialog.
- `src/components/grid/` — virtualised results grid and value inspector.
- `src/components/connections/` — connections page, rows, switcher, URL field.
  Never load a whole result into the webview; use a grid source.
- `src/dev/mockBackend.js` — `?mock` sample backend for reviewing screens in
  a browser (dev only).
- `src-tauri/tests/drivers.rs` — integration tests against real databases.
- `src-tauri/migrations/` — versioned SQLite schema migrations.
- `src-tauri/capabilities/default.json` — keep to least privilege.

## Coding rules

1. Vue components stay under 250 lines; split by responsibility.
2. Colours come only from `src/styles/tokens.css` (`bg-surface`, `text-fg-muted`,
   `border-line`, `bg-danger text-danger-fg`, …). Tailwind's default palette is
   switched off, so `text-white` or `bg-zinc-500` produce nothing, and
   `npm run check:design` fails on hex/rgb values or palette classes anywhere
   in `src/`. New colours are added as tokens in both themes and must pass the
   contrast pairs in `scripts/check-design.js`.
3. Use `BaseDialog` for modals, `confirm()` from `useConfirm` for confirmations,
   `useToasts()` for notifications, `v-tooltip` for tooltips and the common
   inputs for forms. Don't build one-off versions.
4. New user actions are commands registered with `useCommandRegistration`
   (title in Title Case, optional `Mod+…` shortcut). Check the reserved
   shortcuts in `docs/design/workspace.md` first.
5. Icons come from `lucide-vue-next`: 14px in controls, 16px in toolbars,
   `stroke-width` 1.75, `aria-hidden` unless the icon is the only label.
6. Keep business logic out of components: composables, stores and Rust services.
7. The frontend never connects to a database or holds raw credentials.
8. Never show fake data, fake success states or placeholder features that look real.
9. Keep dependencies minimal and record why each new one is needed in `docs/decisions/`.
10. Review `capabilities/` whenever a plugin or native capability is added.
11. Don't refactor unrelated code during a phase.

## Security

- Secrets go in the OS keychain only; never in SQLite, config, logs or errors.
- TLS verification on by default; SSH host keys verified.
- App-generated SQL uses parameters and engine-specific identifier quoting.
- Never log passwords, connection strings, keys or tokens.
- A strict CSP is set in `tauri.conf.json`; don't loosen it without a recorded reason.
- Never weaken a connection's TLS setting automatically. Only the user picks
  Encrypt Without Verifying or Off, and only for that connection.
- Each live connection is one database session; never put a pool behind a
  query editor.
- Cross-platform: no macOS-only assumptions. Use `lib/platform.js` for
  platform words (credential store, file manager), `Mod+…` shortcuts (⌘ or
  Ctrl), `cfg(target_os)` in Rust, and `tauri.<platform>.conf.json` for
  window differences.
- Never retry a statement automatically. Ask before risky statements and
  before writes on Production (`src/lib/sqlSafety.js`).
- Never accept an unknown or changed SSH host key without the user's
  confirmation. Store SSH key file paths, never key contents.

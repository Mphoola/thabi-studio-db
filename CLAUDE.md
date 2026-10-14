# Thabi Studio — notes for Claude

Read `plan.md` before making changes. It is the source of truth for scope,
phases and exit criteria. Work one phase at a time and do not start the next
phase until the current one's exit criteria are verified.

## Product

Local-first macOS desktop client for MySQL, PostgreSQL and SQL Server.
No Thabi backend, accounts, telemetry or cloud dependency. The MVP favours
reliable connections, safe query execution, schema browsing and a good
results grid over breadth of features. See `plan.md` §2 for what is out of scope.

## Current status

- Phase 1 (scaffold and engineering baseline): complete.
- Colour system and workspace design: `docs/design/workspace.md` (proposed).
- Next: Phase 2 (product shell and design system), built to that design.

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
npm run check          # format, ESLint, design check, frontend build, clippy, cargo test
npm run format         # Prettier + rustfmt
npm run lint:fix       # ESLint autofix
npm run tauri build    # release .app/.dmg (slow: LTO)
```

Run `npm run check` after every meaningful change and before calling work done.

## Layout

- `src/components/app/` — app shell (sidebar, status bar, error boundary, About).
- `src/components/common/` — reusable primitives (`BaseButton`, `BaseDialog`,
  `ToastHost`, `ConfirmHost`).
- `src/composables/` — shared state: `useTheme`, `useToasts`, `useConfirm`, `useAppInfo`.
- `src/lib/tauri/` — the only place that calls `invoke`. Add new command
  wrappers to `commands.js`; errors arrive as `{ kind, message }`.
- `src/styles/tokens.css` — the only place colours, type, radii and shadows are defined.
- `src-tauri/src/commands/` — Tauri commands, one module per area.
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
   and `useToasts()` for notifications. Don't build one-off versions.
4. Keep business logic out of components: composables, stores and Rust services.
5. The frontend never connects to a database or holds raw credentials.
6. Never show fake data, fake success states or placeholder features that look real.
7. Keep dependencies minimal and record why each new one is needed in `docs/decisions/`.
8. Review `capabilities/` whenever a plugin or native capability is added.
9. Don't refactor unrelated code during a phase.

## Security

- Secrets go in the OS keychain only; never in SQLite, config, logs or errors.
- TLS verification on by default; SSH host keys verified.
- App-generated SQL uses parameters and engine-specific identifier quoting.
- Never log passwords, connection strings, keys or tokens.
- A strict CSP is set in `tauri.conf.json`; don't loosen it without a recorded reason.

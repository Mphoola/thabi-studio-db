# Thabi Studio

A local-first desktop app for connecting to, exploring and querying MySQL,
PostgreSQL and Microsoft SQL Server databases. Connections go directly from
your Mac to your database servers; there is no account, telemetry or cloud
service.

> Status: early development. The app shell runs, but database features are
> not implemented yet. See [`plan.md`](plan.md) for the roadmap.

## Prerequisites (macOS)

- Xcode Command Line Tools: `xcode-select --install`
- Rust (stable, via [rustup](https://rustup.rs)); developed on 1.96
- Node.js 20.19+ or 22.12+ (required by Vite 8); developed on Node 25

See the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for details.

## Getting started

```bash
npm install
npm run tauri dev
```

The first run compiles the Rust dependencies and can take around 10 minutes.
Later runs start in seconds.

## Scripts

| Command               | What it does                                             |
| --------------------- | -------------------------------------------------------- |
| `npm run tauri dev`   | Run the desktop app with hot reload                      |
| `npm run check`       | Format check, ESLint, frontend build, clippy, Rust tests |
| `npm run format`      | Format JS/Vue/CSS with Prettier and Rust with rustfmt    |
| `npm run lint`        | ESLint (`npm run lint:fix` to autofix)                   |
| `npm run check:rust`  | `cargo clippy` with warnings as errors                   |
| `npm run test:rust`   | `cargo test`                                             |
| `npm run tauri build` | Build a release `.app` / `.dmg` into `src-tauri/target`  |

## Project structure

```text
src/                 Vue 3 frontend (JavaScript)
  components/app/    App shell: sidebar, status bar, error boundary
  components/common/ Reusable UI primitives (buttons, dialogs, toasts)
  composables/       Shared state (theme, toasts, confirm, app info)
  lib/tauri/         Wrappers around Rust commands
  styles/            Tailwind entry and design tokens
src-tauri/           Rust backend (Tauri 2)
docs/decisions/      Architecture decision records
plan.md              MVP plan and progress tracker
CLAUDE.md            Working notes for Claude Code
```

## Recommended editor setup

VS Code with Vue - Official, Tauri and rust-analyzer, or a JetBrains IDE with
the Vue and Rust plugins.

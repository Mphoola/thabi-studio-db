# Thabi Studio

A local-first desktop app for connecting to, exploring and querying MySQL,
PostgreSQL and Microsoft SQL Server databases. Connections go directly from
your computer to your database servers; there is no account, telemetry or cloud
service.

> Status: early development. You can save connections, connect to MySQL,
> PostgreSQL and SQL Server (directly or through SSH), browse databases,
> tables and views, run SQL in query tabs, and browse, copy and export
> results to CSV. Query history and saved queries come next.
> See [`plan.md`](plan.md) for the roadmap.

## Platforms

Thabi Studio runs on **macOS, Windows and Linux**. Passwords are kept in
the platform's credential store: Keychain on macOS, Windows Credential
Manager, or the Secret Service (GNOME Keyring or KWallet) on Linux. CI
builds and checks all three platforms on every push.

## Prerequisites

- Rust (stable, via [rustup](https://rustup.rs)); developed on 1.96
- macOS: Xcode Command Line Tools (`xcode-select --install`)
- Windows: Microsoft C++ Build Tools and WebView2 (preinstalled on Windows 11)
- Linux: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`,
  `libdbus-1-dev`, `patchelf` and `pkg-config` (Debian/Ubuntu names)
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

| Command                    | What it does                                                    |
| -------------------------- | --------------------------------------------------------------- |
| `npm run tauri dev`        | Run the desktop app with hot reload                             |
| `npm run check`            | Format check, ESLint, frontend build, clippy, Rust tests        |
| `npm run format`           | Format JS/Vue/CSS with Prettier and Rust with rustfmt           |
| `npm run lint`             | ESLint (`npm run lint:fix` to autofix)                          |
| `npm run check:rust`       | `cargo clippy` with warnings as errors                          |
| `npm run test:rust`        | `cargo test`                                                    |
| `npm run test:integration` | Driver tests against MySQL, PostgreSQL and SQL Server in Docker |
| `npm run tauri build`      | Build a release `.app` / `.dmg` into `src-tauri/target`         |

## Remote databases and SSH

Connect directly to anything your computer can reach, including over a VPN. For a
database that's only reachable from a bastion, turn on **Connect through
SSH** in the connection settings and sign in with a password, a private key
file or your ssh-agent. The first time you connect to an SSH server, Thabi
Studio shows its fingerprint and asks before trusting it. Keys you already
trust in `~/.ssh/known_hosts` are recognised.

## Reviewing screens without a database

Run `npm run dev` and open `http://localhost:1420/?mock` in a browser. A
development-only sample backend answers every command, so you can click
through connections and the explorer without the native app.

## Integration tests

`npm run test:integration` starts PostgreSQL 16, MySQL 8.4 and SQL Server
2022 and an OpenSSH bastion in Docker (bound to 127.0.0.1), generates a
throwaway password and SSH keys in `tests/integration/` (git-ignored), and
runs `src-tauri/tests/drivers.rs` and `ssh.rs`.
It needs Docker (Docker Desktop on macOS and Windows; SQL Server runs under
Rosetta on Apple Silicon) and a Bash shell (use WSL or Git Bash on Windows).
Stop the databases with `scripts/integration-tests.sh down`.

## Project structure

```text
src/                 Vue 3 frontend (JavaScript)
  components/app/    App shell: sidebar, status bar, error boundary
  components/common/ Reusable UI primitives (buttons, dialogs, toasts)
  composables/       Shared state (theme, toasts, confirm, app info)
  lib/tauri/         Wrappers around Rust commands
  styles/            Tailwind entry and design tokens
src-tauri/           Rust backend (Tauri 2), SQLite migrations, Keychain access
                     Profile and preference metadata are in app data;
                     database and SSH passwords are in the OS credential store.
docs/decisions/      Architecture decision records
plan.md              MVP plan and progress tracker
CLAUDE.md            Working notes for Claude Code
```

## Recommended editor setup

VS Code with Vue - Official, Tauri and rust-analyzer, or a JetBrains IDE with
the Vue and Rust plugins.

# 0012 — Cross-platform: macOS, Windows and Linux

**Status:** accepted · **Date:** 2026-10-15 · Changes plan.md's "macOS first".

The user asked for Thabi Studio to be cross-platform, not just Mac-first.

## Decisions

1. **Credential store per OS** (`credentials.rs`), all through `keyring-core`:
   Keychain (macOS), Windows Credential Manager, and the Secret Service on
   Linux (GNOME Keyring, KWallet), using pure-Rust crypto so no OpenSSL is
   needed. Messages name the store the user knows.
2. **Window chrome.** The overlay title bar with inset traffic lights is
   macOS-only and lives in `tauri.macos.conf.json`; Windows and Linux keep
   their native title bar, with the app's bar below it. Window dragging uses
   `data-tauri-drag-region`, which works everywhere.
3. **Menus.** macOS gets the app menu (About, Settings, Services, Hide,
   Quit) and a Window menu. Windows and Linux put Settings and Quit in File
   and About in Help. Accelerators use `CmdOrCtrl`.
4. **Shortcuts and words.** Shortcuts are written `Mod+…` and show as ⌘ on
   macOS and Ctrl elsewhere. `src/lib/platform.js` supplies platform words:
   "Show in Finder / File Explorer / Folder", "this Mac / this computer",
   and the credential store's name.
5. **SSH agent.** `SSH_AUTH_SOCK` on macOS and Linux; on Windows the
   OpenSSH agent's named pipe, then Pageant. `~/.ssh/known_hosts` is found
   through the home directory on every platform.
6. **Paths.** CSV export, CA and key files use absolute paths checked in
   Rust; the design check script resolves paths with `fileURLToPath` so it
   works on Windows.
7. **CI** (`.github/workflows/ci.yml`): `npm run check` on macOS, Windows
   and Ubuntu, plus the Docker integration tests on Ubuntu, on every push.
   This is how Windows and Linux are verified; they can't be compiled from
   the development Mac (cross-compiling the C dependencies needs each
   platform's SDK).

## Open

- Windows and Linux have only been verified by CI so far, not by hand.
  Phase 10 adds installers and a clean-install test per platform.

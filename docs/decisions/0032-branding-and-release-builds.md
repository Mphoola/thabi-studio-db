# 0032 — Branding, version 1.0.0 and release builds

**Status:** accepted · **Date:** 2026-10-16

## Branding

The app is named for Tabitha Mphoola (see `branding/README.md`). The icon is
a "T" made of a database disc and stacked records, in the app's iris accent,
with a small spark for Tabitha. `branding/logo.svg` is the source; every icon
in `src-tauri/icons/` is generated from its 1024 px render with
`npx tauri icon`. The About window shows the mark, the version, the engines,
the dedication and the privacy note.

## Version

1.0.0 in `package.json`, `src-tauri/tauri.conf.json` and
`src-tauri/Cargo.toml` (the About window reads it from Tauri).

## Release builds

`.github/workflows/release.yml` runs on every push to `main` (and by hand):

- macOS: a universal `.dmg` (Apple silicon and Intel).
- Windows: an NSIS `.exe` installer.

Each run keeps the installers as workflow artifacts and attaches them to a
draft GitHub Release `v<version>` (made by `tauri-apps/tauri-action`), which
is published by hand. The existing `ci.yml` still checks every push on all
three platforms.

The builds are **not code-signed** yet; signing is Phase 10's job. Until
then macOS asks the user to right-click > Open the first time, and Windows
SmartScreen asks before running the installer. Signing needs an Apple
Developer ID (with notarization) and a Windows code-signing certificate,
added as repository secrets.

## Right-click menus

Context menus closed as soon as they opened on macOS: WebKit fires
`contextmenu` on mouse-down, and the release that followed counted as a
click outside the new popover, which light-dismissed it. `DropdownMenu`
now waits for the button to be released before opening a menu at a point.

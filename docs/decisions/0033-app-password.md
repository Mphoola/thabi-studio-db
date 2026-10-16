# 0033 — App password and lock screen

**Status:** accepted · **Date:** 2026-10-16

## Why

Thabi Studio often holds saved passwords for production databases. Anyone
who can use an unlocked computer can open the app and run anything. An
optional app password adds a lock in front of every connection.

## What it does

- Set in Settings › App password. It must be at least 10 characters with
  upper- and lowercase letters. The rules are in `app_lock::strength_error`
  (Rust, enforced) and `src/lib/appPassword.js` (the live checklist).
- When a password is set, the app starts locked. It locks again after the
  idle time chosen in Settings (1–60 minutes, default 15, or only at launch),
  and on **Lock Thabi Studio** / **Lock Now**. Idle time counts activity in
  every window. Locking or unlocking applies to every window.
- Changing or turning it off needs the current password.

## How it is enforced

- **In Rust, not the webview.** `guarded` in `lib.rs` wraps the command
  handler. While locked, every command except the lock screen's own
  (`app_lock::allowed_while_locked`) is refused with kind `locked` before it
  runs. The app counts as locked until the credential store has been read,
  so nothing slips through at startup. `a_locked_app_refuses_commands_before_they_run`
  tests this with real IPC requests.
- The webview mounts the workspace only after the first unlock. A later lock
  covers it with a modal `<dialog>` (opened last, so it sits above other
  dialogs; Escape doesn't close it), makes the workspace `inert` and pauses
  commands and shortcuts. Background calls refused while locked wait and are
  made again after unlock. They never ran, so this isn't a retry of a
  statement.
- **Storage.** Only an Argon2id hash (`argon2` 0.5, default parameters, random
  salt) is kept, in the OS credential store under `app-password`, next to the
  secrets it protects. Nothing is written to SQLite except the idle setting.
- **Guessing.** Five wrong passwords are free; after that each guess waits 30
  seconds, doubling up to 15 minutes. The count is in memory, and each guess
  also costs an Argon2 hash.

## Forgotten password

The password itself can't be recovered. **Forgot password?** on the lock
screen offers, in order:

1. **A recovery code** (like GitHub's). Ten codes such as `4f1c9-b07ae`
   (40 random bits each) are shown once when the password is first set, with
   Copy and Save as Text File. Each works once. Settings › Recovery codes makes
   a new set, which replaces all old ones and needs the current password.
   Changing the password keeps the codes.
2. **Security questions**, optional, set up in Settings with the current
   password. Three different questions (eight suggested, or the person's own);
   all three answers must be right, ignoring case and extra spaces. The error
   doesn't say which answer was wrong. The questions are shown on the lock
   screen, as on any sign-in page.
3. **Reset**, the last resort: it closes every connection and deletes every
   saved database password, SSH password and key passphrase, query history,
   codes and questions. Connections and saved queries stay, so a reset gives
   nothing away that the password guarded.

A code or the answers always come with a new password
(`recover_with_code`, `recover_with_answers`), so the app is never left
unlocked without one. Wrong codes and answers count as wrong guesses, with the
same waits as the password. Codes and answers are stored only as Argon2id
hashes in the credential store (`app-recovery-codes`,
`app-security-questions`; see `app_recovery.rs`). Turning the password off,
or setting it for the first time, removes any old codes and questions.

Security questions are the weaker option, because answers can often be found
or guessed; the setup screen asks for answers only the person would know.

## Not covered

- Someone who can read the user's credential store (an unlocked Keychain)
  can already read the saved database passwords. The app password protects
  the app, not the operating-system account.
- Touch ID / Windows Hello unlock is a possible later addition.

## Dependencies

`argon2` 0.5 and `password-hash` 0.5 (feature `getrandom`, for the OS random
salt), both RustCrypto. `tauri` with the `test` feature as a dev-dependency
only, for the IPC test.

# 0019 — Asking for secrets each time, and SSH keep-alive

**Status:** accepted · **Date:** 2026-10-15 · Phase 11, `plan.md` §7b

A saved password is a liability: it sits in the OS credential store for as
long as the connection exists, and on a shared machine that is not always the
user's machine. "Ask every time" keeps nothing and asks at connect time.

## Decisions

1. **Three separate switches, one per secret.** A profile can ask every time
   for the database password, the SSH password or the key passphrase, in any
   combination, because the reasons differ: a key passphrase might be wanted
   every time while the database password is fine in the Keychain.
2. **Asking means nothing is stored.** Validation drops the typed secret when
   its switch is on, so the two cannot be set at once and the form says so.
   The profile then reports `hasDatabasePassword`, `hasSshPassword` and
   `hasSshKeyPassphrase` as false rather than pretending something is saved.
   The migration adds the three flags with no effect on existing profiles:
   nothing changes until the user asks for it.
3. **The app asks; it never tries and fails.** `connect_connection_profile`
   returns `secret_required` naming the secret it needs, and the frontend shows
   one dialog per secret (a profile may need both a database and an SSH
   password) and retries. Connecting without a password would only produce an
   authentication error that says nothing about the real problem. This mirrors
   how SSH host keys are already handled in 0007 — a named gap, a dialog, a
   retry — rather than inventing a second pattern.
4. **The value lives only for the attempt.** It is passed to the connect call
   as a one-off, dropped when the connection is closed, and never written to
   SQLite, the credential store or a log. An empty value counts as a
   cancellation, so there is no path that sends an empty password.
5. **The credential store is left alone.** Asking every time does not delete
   an existing saved secret on its own — the "Remove saved password" checkbox
   is separate — so turning the switch on and off again is not destructive.
6. **Keep-alive is per connection.** `sshKeepaliveSeconds` (0–3,600, 0 meaning
   the default) is passed to the SSH client, so a connection over a link that
   drops idle sessions can be kept alive without changing every other
   connection. It applies to each hop of a jump chain.

## Verified

- Rust: `ask_every_time_keeps_nothing_in_the_credential_store` saves a
  profile with all three switches on and passwords typed into the form, then
  checks the profile claims no saved secrets, the credential key is empty, no
  stored secret contains the typed values, and the connect parameters carry no
  password. `one_off_secrets_reach_the_session_but_are_not_kept` checks a
  typed password reaches the session and that re-reading the profile still
  finds nothing.
- Frontend: validation accepts asking instead of saving, rejects a typed
  secret while asking, and bounds the keep-alive.
- One bug this found and fixed: an "ask every time" SSH password could not
  even build its connection parameters, because building them demanded a
  stored password. They now carry a placeholder the connect call replaces.

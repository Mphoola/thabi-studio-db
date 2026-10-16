# 0020 — Startup commands per connection

**Status:** accepted · **Date:** 2026-10-15 · Phase 11, `plan.md` §7b

Some sessions need a setting before any of the user's SQL runs: a
`search_path` that isn't the default, a time zone the data is stored in, a
role. Typing it into every tab is exactly the kind of repetition this app
exists to remove.

## Decisions

1. **One statement per line, run in order**, on every new session for that
   connection. Bounded to 20 lines and 4,000 characters, and a line break
   inside a statement is refused — splitting on newlines is the whole rule, so
   a statement that needs two lines has to be one line.
2. **They run wherever a session opens.** `Session::connect` applies them
   right after it applies the read-only guard, which means the connection's own
   session and every query tab's session get them, without either knowing the
   feature exists. This is the case `plan.md` §7b calls out as easy to get
   wrong, and it is covered by a test on a real database.
3. **A failure stops the connection.** Half-configured session state — a
   `search_path` that silently didn't apply — is worse than a clear failure
   with nothing open. The error names the statement and keeps the server's own
   message as its detail, so the user can see which line failed and why.
4. **A read-only connection may only start with session statements.** `SET`,
   `USE`, `RESET` and `BEGIN` are allowed; anything else is rejected at save
   time with a field error. Without this, startup commands would be a way to
   write to a server the user had explicitly made read-only — including on SQL
   Server, where the app check is the only guard (0016).
5. **They are the user's own SQL, shown in the form** rather than hidden
   behind a connection-details panel: the setting is only trustworthy if it is
   visible next to the safe mode it interacts with, and the field's hint
   changes to say that read-only only allows `SET` and `USE`.
6. **`TextInput` grew a `multiline` prop** rather than the dialog building a
   textarea of its own. Later phases need the same thing for SQL scripts, so it
   belongs on the shared primitive.

## Not done here

Startup commands are per connection and run at connect time. They are not run
for the cancel-and-reconnect path in a later sub-part until that exists, and
they say nothing about connections opened before the feature; those simply
have none.

## Verified

- Unit: `startup_commands` splits and bounds the list, rejects control
  characters and oversized input, and refuses anything but session statements
  on a read-only connection (`profiles::validation`).
- Integration: `startup_commands_apply_to_query_tab_sessions` opens a query
  tab with `SET search_path = pg_catalog` and reads `current_schema()` back on
  that tab's own session; `a_failing_startup_command_stops_the_connect` checks
  a bad statement stops the connect and names itself in the error.
- Frontend: the same bounds and read-only rule are checked before saving, so
  the user sees the problem in the form rather than after connecting.

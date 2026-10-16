# 0022 — Per-connection query timeout and row limit

**Status:** accepted · **Date:** 2026-10-15 · Phase 11, `plan.md` §7b

The query timeout and the number of rows a result keeps are app-wide settings.
A report connection wants minutes rather than the app's seconds, and a
production connection may want fewer rows held in memory than the app default.
Either can now be set per connection.

## Decisions

1. **An override, not a replacement.** `query_timeout_seconds` and
   `max_result_rows` on the profile, where 0 means "use the app default", so
   existing connections behave exactly as before and the Settings dialog stays
   the default for everyone. Migration `0012` adds both with a default of 0.
2. **The connection's own value wins, and it is bounded by the same limits the
   app uses** — 86,400 seconds and 1,000,000 rows. `src/lib/limits.js` resolves
   the pair and clamps them, so a value typed into the form cannot reach a
   driver with a limit it will not accept. A nonsense value falls back to the
   app default rather than being passed through.
3. **0 still means "no limit",** which is what the Settings dialog and the
   drivers already treat it as. Resolution therefore returns `null` for an
   absent limit, not 0.
4. **Applied where the query actually runs.** The query runner resolves the
   limits for the tab's connection, and the table data view resolves them for
   the connection it is reading. Nothing else changes: the drivers, the cancel
   path and the row-keeping limit in Rust already take these numbers as
   arguments, so this only chooses which numbers arrive.
5. **An override is visible without opening the form.** A connection row that
   overrides either limit shows a clock icon and the values, with a tooltip
   pointing at Settings for the defaults. Being able to see why a query stopped
   early matters more than the setting itself.

## A note on the tests

While adding these, the same class of mistake appeared twice: the profile
`INSERT` lists 33 columns but bound 31 values, and a mismatch surfaces only as
a generic "Local data couldn't be accessed" at runtime. `profile_statements_
match_the_table` now runs a full create/update/reorder round trip so a
mismatch fails a test that names the statement instead.

## Verified

- `src/lib/limits.js`: app defaults when nothing is set, either value
  overridden independently, 0 as no limit, clamping to what the drivers
  accept, and nonsense falling back rather than being passed on.
- `query_limits_round_trip_and_are_bounded`: both values round-trip through
  create, read and duplicate; 0 stays 0; out-of-range values are rejected by
  field, in camelCase like every other field error.
- `npm run check` and 55 integration tests pass, including
  `timeouts_stop_long_queries`, which is the behaviour these numbers drive.

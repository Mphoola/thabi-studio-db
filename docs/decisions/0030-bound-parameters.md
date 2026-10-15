# 0030 — Bound query parameters

**Status:** accepted · **Date:** 2026-10-15 · Phase 13, `plan.md` §7b

A parameter exists so a value is never part of the statement. `SELECT * FROM
orders WHERE note = $1` with `O'Brien'; DROP TABLE orders; --` stores those
characters, because the value travels beside the SQL and the driver binds it.

## Decisions

1. **A second execution path, not a flag.** `execute_bound` sits beside
   `execute`. Binding needs the prepared statement protocol, which returns typed
   values where the simple protocol returns text — and every engine here uses
   the simple protocol on purpose, each with a comment saying so, because the
   grid wants text. A flag would have made parameterised queries the only
   queries that decode differently.
2. **Each column is decoded as its own type, and an unknown type is an error.**
   Reading everything as a string is not available: PostgreSQL's type check is
   strict identity, so `NUMERIC` cannot be read as a float and an `INT` cannot
   either. A type the decoder does not know is an error naming it, never a
   blank cell — the text path degrades a failed read to NULL, which is right
   there because the simple protocol hands back text and there is nothing to
   misread, and wrong here because it would hide a value the user asked for.
3. **NUMERIC keeps every digit.** `bigdecimal` is a feature on `sqlx-postgres`
   rather than on the `sqlx` umbrella: the umbrella's features pull in
   `sqlx-sqlite`, which links the same SQLite the app already has through
   `rusqlite`, and the two cannot coexist in one binary.
4. **A bound query shows what an unbound one shows.** The test compares the two
   paths directly, which is the only way this property is really checked. It
   caught a real difference: `BigDecimal` renders at the scale the column
   declares and the simple protocol trims trailing zeros, so `…12345000`
   against `…12345`. Trailing zeros are now trimmed in the bound path. Only
   trailing zeros may differ — no digit is added or lost — and the test asserts
   that rather than assuming it.
5. **A NULL parameter binds as text.** PostgreSQL will not cast text to integer
   even for NULL, so a NULL against a numeric column is refused by the server.
   This is a real limit, not a rounding of one: the values form will ask for a
   type, and a parameter typed as NULL will bind as that type.
6. **An engine that cannot bind says so.** SQL Server refuses with
   `not_supported`, and the test asserts the refusal so it cannot quietly start
   dropping values later.

## Known limits

- SQL Server does not bind yet. Tiberius's prepared path returns its own types
  and would need the same decoder written against it.
- A NULL bound for a numeric column is refused by PostgreSQL, per 5.
- Bound queries return the first result set only; `execute` keeps several.

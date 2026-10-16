# 0029 — Editing a query's result

**Status:** accepted · **Date:** 2026-10-15 · Phase 12, `plan.md` §7b

The last Phase 12 exit criterion: editing a result is offered only when it is
safe, and works on all three engines.

## The problem

Editing a result means writing `UPDATE orders SET … WHERE id = …`. For that to
land on the row the user clicked, three things must be true, and none of them
can be assumed:

1. The rows are one row each of **one plainly-read table**.
2. The result carries that table's **whole primary key**.
3. **No key appears twice** in the result.

Anything looser writes to a row somebody is not looking at.

## Decisions

1. **The check is a text check, and it is strict.** `lib/queryEditability.js`
   reads the statement and refuses anything it does not recognise: a second
   SELECT (a subquery), a join, a comma of tables, a CTE, a set operation, a
   parenthesis or call in the select list, `GROUP BY`, `DISTINCT`, a window
   function, more than one statement. A refusal costs a read-only result; a
   wrong "yes" costs data, so the trade is made explicitly.
2. **Aggregates are refused by shape, not by name.** Every selected item must be
   a bare, possibly qualified identifier. `COUNT(*)`, `SUM(x)` and
   `now()` are then refused for the same reason, without listing every function
   each of the three engines has.
3. **The key is compared, not trusted.** A repeated key means the result shows
   the same record twice — a join, a `LIMIT` over a self-join, two pages
   overlapping. Editing either would edit both, and the user cannot tell which
   one they clicked, so the result stays read-only and says so.
4. **No WHERE parsing.** The obvious fourth check — compare the row count
   against `SELECT COUNT(*)` with the same predicate — needs the predicate
   extracted from the text, and a mis-extracted one could pass a join as safe.
   The repeated-key check needs no parsing and catches the same cases.
5. **String literals are blanked before reading; identifier quotes are not.**
   `"orders"`, `` `orders` `` and `[orders]` are how the three engines quote an
   identifier. Blanking them with the string literals hid the very table name
   being looked for, and made `SELECT id FROM "Orders"` look unparseable.
6. **A refusal says why**, in the results bar, so a read-only result is a
   reason rather than a shrug.
7. **The editing is the table editing.** The same staged changes, the same
   preview and confirmation, pointed at the table the statement reads. One
   implementation of "write this row back", not two.

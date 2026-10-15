# 0017 — Connection groups and colours

**Status:** accepted · **Date:** 2026-10-15 · Phase 11, `plan.md` §7b

Safe modes (0016) decide what happens before a statement runs. This is the
other half of the connection story: organising connections, and giving each
one a colour of its own.

## Decisions

1. **A connection has an optional group and an optional colour.** Migration
   `0008` adds `group_name`, `colour` and `sort_order` to
   `connection_profiles`. Both are empty by default, so existing connections
   look and behave exactly as before.
2. **The connections page shows user groups first, then environments.** A
   connection with a group sits under it; anything ungrouped keeps falling
   back to its environment section (Production first), so the page still reads
   sensibly before anyone creates a group. Rust returns profiles in that same
   order — `(group_name = ''), group_name, sort_order, name` — and the page
   preserves it inside each section rather than re-sorting.
3. **Group names are compared case-insensitively** ("Work" and "work" are the
   same group) because they are headings the user types, not identifiers. The
   first spelling written is the one shown.
4. **Ordering is a manual order, not a rename.** Rows drag to reorder inside
   their group, and `Alt+Up` / `Alt+Down` does the same from the keyboard, so
   reordering does not depend on a pointer. `reorder_connection_profiles`
   takes the ids in their new order and writes `sort_order`; unknown ids are
   ignored rather than failing the whole move, and profiles left out keep
   their number so reordering one group never disturbs another.
5. **Only user groups collapse.** Environment sections are a fixed reference
   the app chooses, so they stay open; a user group can be folded away and the
   folded set is remembered in the `collapsed_groups` setting.
6. **Colours come from the existing tag tokens** — red, orange, yellow, green,
   teal, blue, purple, gray — and nothing else. Rust validates the token
   against the same list the frontend offers, so a hand-made request cannot
   invent a colour. A connection with no colour keeps its environment colour,
   which is what the group headings and the rail show today.

## Where the colour shows

The connection rail and the switcher use the custom colour for their dot when
one is set, and the connections page adds a small dot beside the name so a
colour is visible where connections are managed. The environment tag still
decides the Production strip, so colour never hides an environment.

## Verified

- Unit: grouping, case-insensitive groups, the search filter, drop positions,
  colour fallback and the collapse rule (`tests/frontend/connectionGroups.js`
  tests in `tests/frontend/connectionGroups.test.js`).
- Rust: groups and colours round-trip through create, edit and duplicate;
  dragging reorders and is stable; unknown ids are ignored; an unknown colour
  or an over-long group name is rejected by field
  (`profiles::repository::tests`).
- Checked against the running app: migration 0008 applied to a real database
  from Phase 9b without data loss.

## Found while running the app

`ConnectionRail.vue` used `v-tooltip` without importing the directive, so its
tooltips never appeared. Fixed here.

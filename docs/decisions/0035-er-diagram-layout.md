# 0035 — elkjs for the ER diagram

**Status:** accepted · **Date:** 2026-10-17

## Context

Phase 14's last exit criterion is an ER diagram that renders a
hundred-table schema smoothly (`plan.md` §Phase 14). Laying a schema out is
the whole problem: tables become boxes, foreign keys become lines between
them, and the boxes have to be placed so the lines can be read.

Two things make it harder than it sounds:

- A schema of linked tables is not a grid. Tables cluster, and a layout that
  ignores the links produces a picture with no shape in it.
- Foreign keys should join the _columns_ they belong to, not the middle of a
  box. That means every column needs a port, and the layout has to respect
  them.

## Decision

`elkjs`, loaded lazily.

`plan.md` already suggested evaluating "a small layout library such as
`elkjs`". Of the options:

- **elkjs** handles layered graphs with ports properly, which is what a
  foreign-key graph is, and is the only one of the three tested here that
  places 100 ported nodes without complaint.
- **dagre** is smaller and good at layered DAGs, but ports and fixed node
  sizes — both of which this diagram needs — are weaker.
- **Hand-rolling** was considered and rejected. Reimplementing a layered
  layout well is the hard part, and the exit criterion is explicitly about
  scale, where a hand-rolled version is least likely to be good.

The cost is the bundle: elk is loaded on first use of the diagram rather than
at startup, because the editor is the app and the diagram is a page inside it.
`src/lib/erLayout.js` takes the layout function as an argument so the geometry
can be tested without a browser, and `buildGraph` is pure.

## What elk needs, found the hard way

Two requirements are not in its README and neither error explains them:

- `children` must be an **array**. Given a keyed object, elk reports
  `Referenced shape does not exist` for the ports its _own_ edges point at.
- Ports are **children of their node**, each tagged `port: "in"` or
  `"out"`, not a side map.

Both are covered by tests now (`tests/frontend/erLayout.test.js`), including
one that runs 100 tables through the real library.

## Consequences

- `plan.md` §Phase 14's ER diagram task can be met with elkjs, so the exit
  criterion is reachable without inventing layout code.
- Nothing else in the app depends on elk. Removing it costs one module.

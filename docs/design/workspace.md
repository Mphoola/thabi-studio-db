# Workspace design

Status: accepted (2026-10-14). The source of truth for Phase 2 and the UI
work in later phases. Colours come only from `src/styles/tokens.css`.

Based on how TablePlus and DataGrip handle the same jobs:

- TablePlus: one workspace per connection, tabs for opened objects, a
  colour and tag per connection (Local, Testing, Development, Staging,
  Production), Open Anything on ⌘P, and Safe Mode that warns before
  queries are sent.
- DataGrip: an explorer tree, a query console tied to a data source with a
  visible schema context, and results in tabs under the editor.

## UX principles

1. **Context is always visible.** The active connection, its environment
   and the target database/schema are shown in the title bar and the editor
   toolbar before anything runs. You never have to wonder where a query goes.
2. **Production looks different.** Every connection has an environment tag.
   Production adds a red strip under the title bar and a red dot in the
   status bar, and asks before running statements that can change data.
3. **Keyboard first, platform conventions.** Every frequent action has a
   shortcut that follows the platform's habits: ⌘ on macOS (⌘. cancels,
   ⌘, opens settings), Ctrl on Windows and Linux (Ctrl+., Ctrl+,).
   Nothing important needs a mouse.
4. **Calm by default.** Graphite neutrals, one iris accent, hairline
   dividers instead of boxes. Colour is reserved for meaning: environment,
   status, syntax and engine.
5. **Results belong to their query.** Results sit under the editor that
   produced them, one tab per statement, with row count and duration.
6. **Honest states.** Loading, empty, unsupported, disconnected, cancelled
   and failed are all different and all labelled. Show "500 rows fetched"
   when the total isn't known. Never fake success.
7. **Dense but legible.** 13px UI text, 12px monospace data, 28px rows.
   Desktop density, not web-app padding.
8. **Errors that help.** Say what failed, where (connection, line), and
   what to try. The raw driver message stays selectable and copyable.
9. **Destructive actions are named.** Confirmations repeat the object's
   name, focus Cancel, and use a red button labelled with the verb.

## Workspace layout

```text
┌──────────────────────────────────────────────────────────────────────┐
│ ● ● ●   ◉ Production · orders_db ▾          ⌕ Open anything…  ⌘P    │ title bar (38px, draggable)
├──────────────────────────────────────────────────────────────────────┤ red 2px strip on Production
│ EXPLORER │ SAVED │ HISTORY │ orders.sql ● │ customers │ +            │ tab bar (34px)
│──────────┴───────┴─────────│──────────────────────────────────────── │
│ ⌕ Filter            │ ▶ Run ⌘↵  │ orders_db ▾ public ▾ │ Limit 500 ▾ │ editor toolbar (32px)
│ ▾ orders_db         │ 1  SELECT id, customer_email, total…          │
│   ▾ public          │ 2  FROM orders                                 │ SQL editor
│     ▾ Tables (12)   │ 3  WHERE status = 'pending'                    │
│       orders        │═══════════════════════════════════════════════ │ resizable split
│       customers     │ Result 1 · 238 rows · 42 ms │ Messages   ⧉ ⇩  │ results tabs
│     ▸ Views (3)     │ id │ customer_email │ total  │ status │ …      │
│ ▸ analytics         │ 1  │ ana@…          │ 120.00 │ NULL   │        │ results grid
├─────────────────────┴────────────────────────────────────────────────┤
│ ● Production  postgres@db.internal:5432 via SSH      Ready    v0.1.0 │ status bar (24px)
└──────────────────────────────────────────────────────────────────────┘
```

- **Title bar** (unified overlay on macOS; the native title bar on Windows
  and Linux, with this bar below it): traffic lights on macOS, connection
  switcher pill (environment dot, connection name, database), and Open
  Anything on the right. Empty areas drag the window.
- **Sidebar** (240px, resizable 200–360px, ⌘1 to toggle): a segmented
  control for Explorer, Saved and History, a filter field, then the
  lazy-loaded tree. Engine icon and counts on group nodes.
- **Tab bar**: query tabs and opened tables in one row. A dot marks
  unsaved queries; ⌘T new query, ⌘W close.
- **Editor toolbar**: Run (with the shortcut shown), Cancel while running,
  database and schema selectors, row limit.
- **Results panel** (⌘J to toggle, resizable): result tabs per statement,
  a Messages tab, and copy/export actions. The grid uses monospace data,
  zebra rows, `NULL` in subtle italics, and right-aligned numbers.
- **Status bar**: environment dot, connection name and endpoint (with "via
  SSH" when tunnelled), query state, and fetched-row summary.
- **Inspector** (later, ⌥⌘I): row detail and column metadata on the right.

Minimum window 860 × 560. Below 1000px wide the sidebar collapses to icons.

## Environment tags

| Tag         | Token        | Behaviour                                   |
| ----------- | ------------ | ------------------------------------------- |
| Local       | `tag-gray`   | None                                        |
| Development | `tag-blue`   | None                                        |
| Testing     | `tag-teal`   | None                                        |
| Staging     | `tag-orange` | None                                        |
| Production  | `tag-red`    | Red strip; confirm data-changing statements |

## Keyboard shortcuts

| Action                     | Shortcut           |
| -------------------------- | ------------------ |
| Open anything              | ⌘P                 |
| Switch connection          | ⇧⌘K                |
| New connection             | ⌘N                 |
| New query tab              | ⌘T                 |
| Close tab                  | ⌘W                 |
| Run statement or selection | ⌘↵                 |
| Run all                    | ⇧⌘↵                |
| Cancel query               | ⌘.                 |
| Toggle sidebar             | ⌘1                 |
| Toggle results             | ⌘J                 |
| Refresh explorer           | ⌘R                 |
| Settings                   | ⌘,                 |
| New window                 | ⇧⌘N                |
| Explain query              | ⇧⌘E                |
| Format SQL                 | ⌘I (in the editor) |
| Split editor               | ⇧⌘D                |
| Open SQL file              | ⌘O                 |

Connect Selected Connection and Disconnect Selected Connection are available
in Open Anything when a connection is selected.

## Copy

Style: Title Case for buttons, menu items and window
titles; sentence case for messages and labels. An ellipsis (…) on any
command that needs more input. Curly quotes around names. Contractions are
fine. No "Error:" prefixes, no exclamation marks, no apologies. Numbers
with units and thousands separators: "1,204 rows", "42 ms".

| Situation               | Copy                                                                                                                                                               |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| First launch            | **Connect to your first database** · Thabi Studio connects straight from this computer (this Mac on macOS) to MySQL, PostgreSQL or SQL Server. · [New Connection…] |
| No tabs open            | **No open queries** · Press ⌘T to start a query, or ⌘P to open a table.                                                                                            |
| Testing a connection    | Testing connection…                                                                                                                                                |
| Test succeeded          | Connected to PostgreSQL 16.4 in 38 ms.                                                                                                                             |
| Connection refused      | **Couldn't connect to “Production”** · The server refused the connection on port 5432. Check the host and port, and that the database is running.                  |
| Wrong password          | **Couldn't connect to “Production”** · The server rejected the user name or password.                                                                              |
| Disconnected            | Disconnected from “Production”. [Reconnect]                                                                                                                        |
| Running                 | Running… 3.2 s · [Cancel ⌘.]                                                                                                                                       |
| Rows returned           | 238 rows · 42 ms                                                                                                                                                   |
| Bounded fetch           | First 500 rows fetched. [Fetch More]                                                                                                                               |
| No rows                 | Query ran. No rows returned.                                                                                                                                       |
| Statement without rows  | 12 rows affected · 18 ms                                                                                                                                           |
| Cancelled               | Query cancelled after 3.2 s.                                                                                                                                       |
| Query failed            | **Query failed on “Production” (orders_db)** · then the driver message in monospace, with the line number if known.                                                |
| Unsupported metadata    | SQL Server doesn't report this for views.                                                                                                                          |
| Empty schema            | No tables in “public”.                                                                                                                                             |
| Data-changing on prod   | **Run on Production?** · This statement can change data in “orders_db”. · [Cancel] [Run]                                                                           |
| No WHERE clause         | **Update every row in “orders”?** · This UPDATE has no WHERE clause. · [Cancel] [Update All Rows]                                                                  |
| Delete connection       | **Delete “Production”?** · Its saved password is also removed from your keychain. This can't be undone. · [Cancel] [Delete]                                        |
| Certificate not trusted | **The server's certificate couldn't be verified.** · Check the host name, choose its CA certificate, or, for a server you trust, use Encrypt Without Verifying.    |
| TLS off, remote host    | Not encrypted. Passwords and data cross the network in plain text.                                                                                                 |
| Export done             | Exported 238 rows to orders.csv. [Show in Finder]                                                                                                                  |
| Copied                  | Copied 3 rows.                                                                                                                                                     |

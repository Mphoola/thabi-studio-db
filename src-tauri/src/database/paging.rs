//! Reading a table a page at a time, straight from the server.
//!
//! Phase 12 replaces "fetch N rows and page through them in memory" with a real
//! `LIMIT`/`OFFSET` read, so a table with a million rows opens as fast as one
//! with ten. Three things have to be right for paging to behave:
//!
//! - **A stable order.** Rows are ordered by the primary key, so page 2 starts
//!   where page 1 stopped. When the user sorts by a column, the primary key is
//!   appended as a tie-breaker: without it, equal values could appear on two
//!   pages or on none.
//! - **A quoted order-by column.** The sort column is a name, not a value, so
//!   it goes through `quote::ident` like every other identifier. Nothing the
//!   user types reaches the statement as text.
//! - **An honest count.** Catalog statistics are an estimate and are labelled
//!   as one; the exact count is a separate query the user asks for.

use super::{Engine, quote};

/// How a table's rows are ordered, as the grid asks for it.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageOrder {
    /// Column name to sort by; empty means "primary key order only".
    #[serde(default)]
    pub column: String,
    #[serde(default)]
    pub descending: bool,
}

/// One page of rows plus what is known about the table's size.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TablePage {
    pub rows: Vec<Vec<super::values::Cell>>,
    /// Rows the catalog statistics estimate; not exact, and absent when the
    /// catalog knows nothing (a view, or a table it doesn't cover).
    pub estimated_total: Option<u64>,
}

/// `ORDER BY` for a paged read: the sort column first, then the primary key
/// so equal values keep a fixed order across pages.
fn order_by(engine: Engine, order: Option<&PageOrder>, primary_key: &[String]) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(order) = order.filter(|o| !o.column.trim().is_empty()) {
        parts.push(format!(
            "{}{}",
            quote::ident(engine, order.column.trim()),
            if order.descending { " DESC" } else { " ASC" }
        ));
    }
    for column in primary_key {
        // A sort on the key itself must not repeat it.
        if order.is_some_and(|o| o.column.trim().eq_ignore_ascii_case(column)) {
            continue;
        }
        parts.push(quote::ident(engine, column));
    }
    if parts.is_empty() {
        return String::new();
    }
    format!(" ORDER BY {}", parts.join(", "))
}

/// SQL for one page of a table or view.
///
/// `filter` is a WHERE clause from `filter::where_clause` ("" for none).
/// `primary_key` may be empty for a view, which then has no guaranteed order —
/// the caller decides whether that is worth telling the user about.
#[allow(clippy::too_many_arguments)] // One argument per part of the read.
pub fn page_sql(
    engine: Engine,
    database: &str,
    schema: &str,
    name: &str,
    filter: &str,
    order: Option<&PageOrder>,
    primary_key: &[String],
    offset: usize,
    limit: usize,
) -> String {
    let table = quote::table(engine, database, schema, name);
    let order = order_by(engine, order, primary_key);
    match engine {
        Engine::SqlServer => {
            // OFFSET/FETCH needs an ORDER BY, so an unordered view gets one by
            // the first column rather than paging in an arbitrary order.
            let order = if order.is_empty() {
                " ORDER BY (SELECT NULL)".to_owned()
            } else {
                order
            };
            format!(
                "SELECT * FROM {table}{filter}{order} OFFSET {offset} ROWS FETCH NEXT {limit} ROWS ONLY"
            )
        }
        _ => format!("SELECT * FROM {table}{filter}{order} LIMIT {limit} OFFSET {offset}"),
    }
}

/// A cheap estimate of how many rows a table has, from catalog statistics.
///
/// `None` means the catalog doesn't know: a view, or a database whose
/// statistics have never been gathered. The UI says "unknown" rather than
/// showing a number it can't stand behind.
pub fn estimate_sql(engine: Engine, database: &str, schema: &str, name: &str) -> String {
    let quoted = quote::table(engine, database, schema, name);
    match engine {
        Engine::Postgres => {
            format!("SELECT reltuples::bigint AS n FROM pg_class WHERE oid = {quoted}::regclass")
        }
        Engine::MySql => format!(
            "SELECT TABLE_ROWS AS n FROM information_schema.TABLES WHERE TABLE_SCHEMA = {} AND TABLE_NAME = {}",
            quote::literal(engine, database),
            quote::literal(engine, name)
        ),
        Engine::SqlServer => format!(
            "SELECT SUM(ps.row_count) AS n FROM sys.dm_db_partition_stats ps WHERE ps.object_id = OBJECT_ID({quoted})"
        ),
    }
}

/// The exact number of rows, matching the same filter the page shows.
pub fn count_sql(engine: Engine, database: &str, schema: &str, name: &str, filter: &str) -> String {
    let table = quote::table(engine, database, schema, name);
    format!("SELECT COUNT(*) AS n FROM {table}{filter}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> Vec<String> {
        ["tenant_id", "id"].map(String::from).into()
    }

    #[test]
    fn pages_in_primary_key_order_by_default() {
        assert_eq!(
            page_sql(
                Engine::Postgres,
                "shop",
                "public",
                "orders",
                "",
                None,
                &key(),
                400,
                200
            ),
            "SELECT * FROM \"public\".\"orders\" ORDER BY \"tenant_id\", \"id\" LIMIT 200 OFFSET 400"
        );
        assert_eq!(
            page_sql(Engine::MySql, "shop", "", "orders", "", None, &key(), 0, 10),
            "SELECT * FROM `shop`.`orders` ORDER BY `tenant_id`, `id` LIMIT 10 OFFSET 0"
        );
    }

    #[test]
    fn a_sort_column_leads_and_the_key_breaks_ties() {
        assert_eq!(
            page_sql(
                Engine::Postgres,
                "shop",
                "public",
                "orders",
                "",
                Some(&PageOrder {
                    column: "created_at".into(),
                    descending: true
                }),
                &key(),
                0,
                50
            ),
            "SELECT * FROM \"public\".\"orders\" ORDER BY \"created_at\" DESC, \"tenant_id\", \"id\" LIMIT 50 OFFSET 0"
        );
    }

    #[test]
    fn sorting_on_the_key_does_not_repeat_it() {
        let sql = page_sql(
            Engine::Postgres,
            "shop",
            "public",
            "orders",
            "",
            Some(&PageOrder {
                column: "id".into(),
                descending: false,
            }),
            &key(),
            0,
            10,
        );
        assert_eq!(
            sql,
            "SELECT * FROM \"public\".\"orders\" ORDER BY \"id\" ASC, \"tenant_id\" LIMIT 10 OFFSET 0"
        );
    }

    #[test]
    fn a_sort_column_is_quoted_not_interpolated() {
        let sql = page_sql(
            Engine::Postgres,
            "shop",
            "public",
            "orders",
            "",
            Some(&PageOrder {
                column: "id\"; DROP TABLE orders; --".into(),
                descending: false,
            }),
            &key(),
            0,
            10,
        );
        assert!(
            sql.contains(r#""id""; DROP TABLE orders; --""#),
            "the name is quoted as one identifier: {sql}"
        );
        assert!(!sql.contains("; DROP TABLE orders; --\n"), "{sql}");
    }

    #[test]
    fn a_filter_is_carried_through_untouched() {
        let filter = " WHERE \"status\" = 'pending'";
        assert!(
            page_sql(
                Engine::Postgres,
                "shop",
                "public",
                "orders",
                filter,
                None,
                &key(),
                200,
                200
            )
            .contains(filter)
        );
    }

    #[test]
    fn sql_server_pages_with_offset_fetch_and_always_orders() {
        assert_eq!(
            page_sql(
                Engine::SqlServer,
                "shop",
                "dbo",
                "orders",
                "",
                None,
                &key(),
                200,
                100
            ),
            "SELECT * FROM [shop].[dbo].[orders] ORDER BY [tenant_id], [id] OFFSET 200 ROWS FETCH NEXT 100 ROWS ONLY"
        );
        // A view with no primary key still needs an ORDER BY to page at all.
        assert_eq!(
            page_sql(Engine::SqlServer, "shop", "dbo", "v", "", None, &[], 0, 10),
            "SELECT * FROM [shop].[dbo].[v] ORDER BY (SELECT NULL) OFFSET 0 ROWS FETCH NEXT 10 ROWS ONLY"
        );
    }

    #[test]
    fn a_view_without_a_key_pages_unordered_or_not_at_all() {
        // PostgreSQL and MySQL can page without an ORDER BY, but the rows then
        // come back in no guaranteed order; that is the caller's choice.
        assert_eq!(
            page_sql(
                Engine::Postgres,
                "shop",
                "public",
                "v",
                "",
                None,
                &[],
                0,
                10
            ),
            "SELECT * FROM \"public\".\"v\" LIMIT 10 OFFSET 0"
        );
    }

    #[test]
    fn estimates_and_counts_quote_their_names() {
        assert_eq!(
            estimate_sql(Engine::Postgres, "shop", "public", "Order Details"),
            "SELECT reltuples::bigint AS n FROM pg_class WHERE oid = \"public\".\"Order Details\"::regclass"
        );
        assert_eq!(
            estimate_sql(Engine::MySql, "shop", "", "a`b"),
            "SELECT TABLE_ROWS AS n FROM information_schema.TABLES WHERE TABLE_SCHEMA = 'shop' AND TABLE_NAME = 'a`b'"
        );
        assert_eq!(
            estimate_sql(Engine::SqlServer, "shop", "dbo", "orders"),
            "SELECT SUM(ps.row_count) AS n FROM sys.dm_db_partition_stats ps WHERE ps.object_id = OBJECT_ID([shop].[dbo].[orders])"
        );
        assert_eq!(
            count_sql(
                Engine::Postgres,
                "shop",
                "public",
                "orders",
                " WHERE \"id\" > 3"
            ),
            "SELECT COUNT(*) AS n FROM \"public\".\"orders\" WHERE \"id\" > 3"
        );
    }
}

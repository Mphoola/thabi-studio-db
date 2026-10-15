//! Row filters for table data (the filter bar above a table's rows).
//!
//! A filter is the user's own WHERE clause, built from column, operator and
//! value, or typed as raw SQL. It runs only on that table tab's session, like
//! SQL from the editor, so values are written as escaped literals here rather
//! than bound: the object-tab session runs text, and the user can already run
//! any SQL there. Column names always go through `quote::ident`.

use serde::Deserialize;

use super::{Engine, quote};
use crate::errors::AppError;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RowFilter {
    /// Ignored for `raw`.
    #[serde(default)]
    pub column: String,
    pub op: String,
    #[serde(default)]
    pub value: String,
}

use quote::literal;

/// The column as text, for LIKE on any column type.
fn as_text(engine: Engine, column: &str) -> String {
    match engine {
        Engine::Postgres => format!("CAST({column} AS TEXT)"),
        Engine::MySql => column.to_owned(),
        Engine::SqlServer => format!("CAST({column} AS NVARCHAR(MAX))"),
    }
}

fn invalid(message: &str) -> AppError {
    AppError::new("invalid_filter", message.to_owned())
}

fn condition(engine: Engine, filter: &RowFilter) -> Result<String, AppError> {
    if filter.op == "raw" {
        let raw = filter.value.trim();
        if raw.is_empty() {
            return Err(invalid("Type a condition for the raw SQL filter."));
        }
        if raw.contains(';') {
            return Err(invalid("A filter is one condition; remove the semicolon."));
        }
        return Ok(format!("({raw})"));
    }
    if filter.column.is_empty() {
        return Err(invalid("Choose a column for each filter."));
    }
    let column = quote::ident(engine, &filter.column);
    let value = &filter.value;
    let like = |pattern: String, negate: bool| {
        format!(
            "{} {}LIKE {}",
            as_text(engine, &column),
            if negate { "NOT " } else { "" },
            literal(engine, &pattern)
        )
    };
    let list = || {
        let items: Vec<_> = value
            .split(',')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| literal(engine, v))
            .collect();
        if items.is_empty() {
            Err(invalid("List one or more values, separated by commas."))
        } else {
            Ok(items.join(", "))
        }
    };
    let compare = |op: &str| format!("{column} {op} {}", literal(engine, value));
    Ok(match filter.op.as_str() {
        "eq" => compare("="),
        "ne" => compare("<>"),
        "lt" => compare("<"),
        "le" => compare("<="),
        "gt" => compare(">"),
        "ge" => compare(">="),
        "contains" => like(format!("%{value}%"), false),
        "not_contains" => like(format!("%{value}%"), true),
        "starts_with" => like(format!("{value}%"), false),
        "ends_with" => like(format!("%{value}"), false),
        "in" => format!("{column} IN ({})", list()?),
        "not_in" => format!("{column} NOT IN ({})", list()?),
        "is_null" => format!("{column} IS NULL"),
        "not_null" => format!("{column} IS NOT NULL"),
        _ => return Err(invalid("That filter operator isn't supported.")),
    })
}

/// " WHERE a AND b", or "" when there are no filters.
pub fn where_clause(engine: Engine, filters: &[RowFilter]) -> Result<String, AppError> {
    if filters.is_empty() {
        return Ok(String::new());
    }
    let parts = filters
        .iter()
        .map(|f| condition(engine, f))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(format!(" WHERE {}", parts.join(" AND ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(column: &str, op: &str, value: &str) -> RowFilter {
        RowFilter {
            column: column.into(),
            op: op.into(),
            value: value.into(),
        }
    }

    #[test]
    fn builds_quoted_conditions_for_each_engine() {
        let filters = [f("id", "eq", "71"), f("name", "contains", "o'b")];
        assert_eq!(
            where_clause(Engine::Postgres, &filters).unwrap(),
            r#" WHERE "id" = '71' AND CAST("name" AS TEXT) LIKE '%o''b%'"#
        );
        assert_eq!(
            where_clause(Engine::MySql, &[f("a`b", "eq", r"x\'")]).unwrap(),
            r" WHERE `a``b` = 'x\\'''"
        );
        assert_eq!(
            where_clause(Engine::SqlServer, &[f("x", "starts_with", "Ü")]).unwrap(),
            " WHERE CAST([x] AS NVARCHAR(MAX)) LIKE N'Ü%'"
        );
    }

    #[test]
    fn lists_nulls_and_raw() {
        assert_eq!(
            where_clause(
                Engine::Postgres,
                &[
                    f("id", "in", "1, 2,,3"),
                    f("note", "is_null", "ignored"),
                    f("", "raw", " a > b "),
                ]
            )
            .unwrap(),
            r#" WHERE "id" IN ('1', '2', '3') AND "note" IS NULL AND (a > b)"#
        );
        assert_eq!(where_clause(Engine::Postgres, &[]).unwrap(), "");
    }

    #[test]
    fn rejects_incomplete_or_unsafe_filters() {
        for bad in [
            f("", "raw", "  "),
            f("", "raw", "1=1; DROP TABLE x"),
            f("", "eq", "1"),
            f("id", "in", " , "),
            f("id", "between", "1"),
        ] {
            assert_eq!(
                where_clause(Engine::Postgres, &[bad]).unwrap_err().kind,
                "invalid_filter"
            );
        }
    }
}

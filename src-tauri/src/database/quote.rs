//! Identifier quoting for SQL the app generates (never for user SQL), plus
//! string literals for the SQL a user builds and sees: row filters and
//! staged row edits (see docs/decisions/0014 and 0015). Metadata queries
//! bind their values instead.

use super::Engine;

/// Quotes one identifier: "name" (PostgreSQL), `name` (MySQL) or
/// [name] (SQL Server), doubling the closing quote character inside.
pub fn ident(engine: Engine, name: &str) -> String {
    match engine {
        Engine::Postgres => format!("\"{}\"", name.replace('"', "\"\"")),
        Engine::MySql => format!("`{}`", name.replace('`', "``")),
        Engine::SqlServer => format!("[{}]", name.replace(']', "]]")),
    }
}

/// Quotes and joins name parts, skipping empty ones: db.schema.table.
pub fn qualified(engine: Engine, parts: &[&str]) -> String {
    parts
        .iter()
        .filter(|part| !part.is_empty())
        .map(|part| ident(engine, part))
        .collect::<Vec<_>>()
        .join(".")
}

/// A string literal with the engine's escaping, for SQL the user sees and
/// owns (row filters, staged row edits). The server converts it to the
/// column's type, so `'71'` works for an integer column.
pub fn literal(engine: Engine, value: &str) -> String {
    match engine {
        Engine::Postgres => format!("'{}'", value.replace('\'', "''")),
        Engine::MySql => format!("'{}'", value.replace('\\', "\\\\").replace('\'', "''")),
        Engine::SqlServer => format!("N'{}'", value.replace('\'', "''")),
    }
}

/// A table's name as each engine addresses it from a session in `database`.
pub fn table(engine: Engine, database: &str, schema: &str, name: &str) -> String {
    match engine {
        Engine::Postgres => qualified(engine, &[schema, name]),
        Engine::MySql => qualified(engine, &[database, name]),
        Engine::SqlServer => qualified(engine, &[database, schema, name]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_awkward_names_for_each_engine() {
        let cases = [
            (
                "Order Details",
                "\"Order Details\"",
                "`Order Details`",
                "[Order Details]",
            ),
            ("select", "\"select\"", "`select`", "[select]"),
            ("MixedCase", "\"MixedCase\"", "`MixedCase`", "[MixedCase]"),
            ("ünïcödé", "\"ünïcödé\"", "`ünïcödé`", "[ünïcödé]"),
            ("a\"b", "\"a\"\"b\"", "`a\"b`", "[a\"b]"),
            ("a`b", "\"a`b\"", "`a``b`", "[a`b]"),
            ("a]b", "\"a]b\"", "`a]b`", "[a]]b]"),
        ];
        for (name, pg, my, ms) in cases {
            assert_eq!(ident(Engine::Postgres, name), pg);
            assert_eq!(ident(Engine::MySql, name), my);
            assert_eq!(ident(Engine::SqlServer, name), ms);
        }
    }

    #[test]
    fn qualified_names_skip_empty_parts() {
        assert_eq!(
            qualified(Engine::SqlServer, &["shop", "dbo", "Order Details"]),
            "[shop].[dbo].[Order Details]"
        );
        assert_eq!(
            qualified(Engine::MySql, &["shop", "", "orders"]),
            "`shop`.`orders`"
        );
    }
}

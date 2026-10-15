//! Identifier quoting for SQL the app generates (never for user SQL).
//! Values always go through bind parameters; identifiers can't, so they are
//! quoted here with each engine's rules.

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

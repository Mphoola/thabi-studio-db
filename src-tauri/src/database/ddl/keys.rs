//! Indexes and keys, which live in different places on each engine.

use super::super::{Engine, quote};
use super::table::{ForeignKey, foreign_key_clause, join_idents};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexSpec {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyChange {
    SetPrimaryKey {
        name: String,
        columns: Vec<String>,
    },
    /// Carries the constraint's name: all three engines drop it by name, and
    /// a guessed name would fail with a constraint that does not exist.
    DropPrimaryKey {
        name: String,
    },
    AddForeignKey(ForeignKey),
    DropForeignKey {
        name: String,
    },
    AddCheck {
        name: String,
        expression: String,
    },
    DropCheck {
        name: String,
    },
}

/// Statements for an index, which lives in a different place on each engine.
pub fn create_index(
    engine: Engine,
    database: &str,
    schema: &str,
    table: &str,
    index: &IndexSpec,
) -> String {
    let name = quote::ident(engine, &index.name);
    let columns = join_idents(engine, index.columns.iter().map(String::as_str));
    match engine {
        // MySQL keeps indexes inside the table's own statement.
        Engine::MySql => format!(
            "ALTER TABLE {} ADD {}INDEX {} ({});",
            quote::table(engine, database, schema, table),
            if index.unique { "UNIQUE " } else { "" },
            name,
            columns
        ),
        _ => format!(
            "CREATE {}INDEX {name} ON {} ({columns});",
            if index.unique { "UNIQUE " } else { "" },
            quote::table(engine, database, schema, table)
        ),
    }
}

pub fn drop_index(
    engine: Engine,
    database: &str,
    schema: &str,
    table: &str,
    index: &str,
) -> String {
    let name = quote::ident(engine, index);
    match engine {
        Engine::MySql => format!(
            "ALTER TABLE {} DROP INDEX {name};",
            quote::table(engine, database, schema, table)
        ),
        // PostgreSQL and SQL Server drop an index without naming the table.
        _ => format!("DROP INDEX {name};"),
    }
}

/// Statements for a key or constraint.
pub fn alter_key(
    engine: Engine,
    database: &str,
    schema: &str,
    table: &str,
    change: &KeyChange,
) -> Vec<String> {
    let target = quote::table(engine, database, schema, table);
    match change {
        KeyChange::SetPrimaryKey { name, columns } => {
            let columns = join_idents(engine, columns.iter().map(String::as_str));
            if engine == Engine::MySql && !name.trim().is_empty() {
                return vec![format!(
                    "ALTER TABLE {target} ADD CONSTRAINT {} PRIMARY KEY ({columns});",
                    quote::ident(engine, name.trim())
                )];
            }
            vec![format!("ALTER TABLE {target} ADD PRIMARY KEY ({columns});")]
        }
        KeyChange::DropPrimaryKey { name } => vec![format!(
            "ALTER TABLE {target} DROP CONSTRAINT {};",
            quote::ident(engine, name)
        )],
        KeyChange::AddForeignKey(key) => vec![format!(
            "ALTER TABLE {target} ADD {};",
            foreign_key_clause(engine, key, 0).trim_start()
        )],
        // Every engine drops a constraint by name.
        KeyChange::DropForeignKey { name } | KeyChange::DropCheck { name } => {
            vec![format!(
                "ALTER TABLE {target} DROP CONSTRAINT {};",
                quote::ident(engine, name)
            )]
        }
        KeyChange::AddCheck { name, expression } => vec![format!(
            "ALTER TABLE {target} ADD CONSTRAINT {} CHECK ({});",
            quote::ident(engine, name),
            expression.trim()
        )],
    }
}

#[cfg(test)]
#[path = "tests/keys.rs"]
mod tests;

//! Changing a table that already exists: one action is one or more statements.

use super::super::{Engine, quote};
use super::table::{ColumnSpec, ForeignKey};

/// A change to a table that already exists.
///
/// One of these is one thing the person asked for, which may be more than one
/// statement: SQL Server changes a column's type in a statement of its own,
/// and cannot do it in the same one that changes nullability.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub enum Change {
    AddColumn(ColumnSpec),
    DropColumn {
        name: String,
    },
    RenameColumn {
        from: String,
        to: String,
        /// Carried so a rename and a re-type stay one action.
        spec: ColumnSpec,
    },
    SetType {
        name: String,
        data_type: String,
    },
    SetNullable {
        name: String,
        nullable: bool,
        /// Carried because MySQL and SQL Server restate the column to change
        /// nullability, and the restatement needs its type.
        data_type: String,
    },
    /// An empty default drops it.
    SetDefault {
        name: String,
        default: String,
    },
    SetComment {
        name: String,
        comment: String,
        /// Carried for the same reason as `SetNullable`.
        data_type: String,
    },
    /// PostgreSQL and SQL Server only; MySQL has no column order to speak of
    /// outside `FIRST`/`AFTER`, which is a separate change.
    MoveColumn {
        name: String,
        after: Option<String>,
    },
}

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

/// `ALTER TABLE` statements for one change, in the order they must run.
///
/// A `Vec` because SQL Server needs `ALTER COLUMN` on its own, and because
/// dropping a key and adding one of a different shape are two statements.
pub fn alter_column(
    engine: Engine,
    database: &str,
    schema: &str,
    table: &str,
    change: &Change,
) -> Vec<String> {
    let target = quote::table(engine, database, schema, table);
    match change {
        Change::AddColumn(column) => {
            // SQL Server has no COLUMN keyword here: `ALTER TABLE t ADD c INT`.
            let keyword = if engine == Engine::SqlServer {
                "ADD"
            } else {
                "ADD COLUMN"
            };
            vec![format!(
                "ALTER TABLE {target} {keyword} {};",
                column.column_line(engine)
            )]
        }
        Change::DropColumn { name } => {
            vec![format!(
                "ALTER TABLE {target} DROP COLUMN {};",
                quote::ident(engine, name)
            )]
        }
        Change::RenameColumn { from, to, spec } => match engine {
            // MySQL renames by restating the whole column.
            Engine::MySql => vec![format!(
                "ALTER TABLE {target} CHANGE COLUMN {} {};",
                quote::ident(engine, from),
                spec.column_line(engine)
            )],
            // SQL Server has no column rename in DDL; it goes through a
            // procedure and says which kind of object it is renaming.
            Engine::SqlServer => vec![
                format!(
                    "EXEC sp_rename N'{}.{}.{}.{}', N'{}', N'COLUMN';",
                    quote::ident(engine, database),
                    quote::ident(engine, schema),
                    quote::ident(engine, table),
                    quote::ident(engine, from),
                    if to.is_empty() { from.as_str() } else { to }
                ),
                // The type still needs restating, since sp_rename does not
                // touch it.
                format!(
                    "ALTER TABLE {target} ALTER COLUMN {};",
                    spec.column_line(engine)
                ),
            ],
            Engine::Postgres => vec![format!(
                "ALTER TABLE {target} RENAME COLUMN {} TO {};",
                quote::ident(engine, from),
                quote::ident(engine, to)
            )],
        },
        Change::SetType { name, data_type } => match engine {
            // Postgres will not cast implicitly; the USING clause says how,
            // and a cast that cannot work is the server's to refuse.
            Engine::Postgres => vec![format!(
                "ALTER TABLE {target} ALTER COLUMN {} TYPE {} USING {}::{};",
                quote::ident(engine, name),
                data_type.trim(),
                quote::ident(engine, name),
                data_type.trim()
            )],
            _ => vec![format!(
                "ALTER TABLE {target} ALTER COLUMN {} {};",
                quote::ident(engine, name),
                data_type.trim()
            )],
        },
        Change::SetNullable {
            name,
            nullable,
            data_type,
        } => {
            let column = quote::ident(engine, name);
            let word = if *nullable {
                "DROP NOT NULL"
            } else {
                "SET NOT NULL"
            };
            match engine {
                // MySQL has no SET NOT NULL: nullability is part of the
                // column's restatement.
                Engine::MySql => vec![format!(
                    "ALTER TABLE {target} MODIFY COLUMN {column} {} {};",
                    data_type.trim(),
                    if *nullable { "NULL" } else { "NOT NULL" }
                )],
                Engine::Postgres => vec![format!(
                    "ALTER TABLE {target} ALTER COLUMN {column} {word};"
                )],
                // SQL Server needs a type with every nullability change.
                Engine::SqlServer => vec![format!(
                    "ALTER TABLE {target} ALTER COLUMN {column} {} {};",
                    data_type.trim(),
                    if *nullable { "NULL" } else { "NOT NULL" }
                )],
            }
        }
        Change::SetDefault { name, default } => {
            let column = quote::ident(engine, name);
            if default.trim().is_empty() {
                vec![match engine {
                    Engine::MySql => {
                        format!("ALTER TABLE {target} ALTER COLUMN {column} DROP DEFAULT;")
                    }
                    _ => format!("ALTER TABLE {target} ALTER COLUMN {column} DROP DEFAULT;"),
                }]
            } else {
                vec![format!(
                    "ALTER TABLE {target} ALTER COLUMN {column} SET DEFAULT {};",
                    default.trim()
                )]
            }
        }
        Change::SetComment {
            name,
            comment,
            data_type,
        } => match engine {
            // Only MySQL keeps a comment on the column itself.
            Engine::MySql => vec![format!(
                "ALTER TABLE {target} MODIFY COLUMN {} {} COMMENT {};",
                quote::ident(engine, name),
                data_type.trim(),
                if comment.trim().is_empty() {
                    "''".to_owned()
                } else {
                    quote::literal(engine, comment.trim())
                }
            )],
            // PostgreSQL comments are separate statements on the object.
            Engine::Postgres => {
                let column = quote::ident(engine, name);
                vec![format!(
                    "COMMENT ON COLUMN {target}.{column} IS {};",
                    if comment.trim().is_empty() {
                        "NULL".to_owned()
                    } else {
                        quote::literal(engine, comment.trim())
                    }
                )]
            }
            // SQL Server has no column comment; a description is an extended
            // property, and changing one means dropping the old first.
            _ => {
                let described = if comment.trim().is_empty() {
                    "NULL".to_owned()
                } else {
                    quote::literal(engine, comment.trim())
                };
                vec![format!(
                    "EXEC sp_updateextendedproperty N'MS_Description', {}, N'SCHEMA', N'{database}', \
                     N'TABLE', N'{table}', N'COLUMN', N'{name}';",
                    described
                )]
            }
        },
        Change::MoveColumn { name, after } => match engine {
            // Column order is a MySQL concept in DDL.
            Engine::MySql => {
                let column = quote::ident(engine, name);
                let position = match after {
                    Some(previous) => format!(" AFTER {}", quote::ident(engine, previous)),
                    None => " FIRST".to_owned(),
                };
                vec![format!(
                    "ALTER TABLE {target} MODIFY COLUMN {column}{position};"
                )]
            }
            _ => Vec::new(),
        },
    }
}

#[cfg(test)]
#[path = "tests/alter.rs"]
mod tests;

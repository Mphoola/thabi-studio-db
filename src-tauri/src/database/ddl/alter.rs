//! Changing a table that already exists: one action is one or more statements.

use super::super::{Engine, quote};
use super::current::{Current, MySqlColumn};
use super::table::{ColumnSpec, ForeignKey};

/// A change to a table that already exists.
///
/// One of these is one thing the person asked for, which may be more than one
/// statement: SQL Server changes a column's type in a statement of its own,
/// and cannot do it in the same one that changes nullability.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(rename_all_fields = "camelCase")]
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
    alter_column_with(engine, database, schema, table, change, &Current::default())
}

/// Like [`alter_column`], restating columns from what the server says they
/// are now (see [`super::current`]), so a change keeps what it doesn't touch.
pub fn alter_column_with(
    engine: Engine,
    database: &str,
    schema: &str,
    table: &str,
    change: &Change,
    current: &Current,
) -> Vec<String> {
    let target = quote::table(engine, database, schema, table);
    if engine == Engine::MySql
        && let Some(statements) = mysql_restated(&target, change, current)
    {
        return statements;
    }
    if engine == Engine::SqlServer
        && let Some(statements) = sql_server(schema, table, &target, change, current)
    {
        return statements;
    }
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
            // SQL Server's rename is in `sql_server`.
            _ => vec![format!(
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
            // MySQL spells a restated column MODIFY; ALTER COLUMN is
            // PostgreSQL's, and SQL Server happens to use it too.
            Engine::MySql => vec![format!(
                "ALTER TABLE {target} MODIFY COLUMN {} {};",
                quote::ident(engine, name),
                data_type.trim()
            )],
            Engine::SqlServer => vec![format!(
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
            // (SQL Server's extended property is in `sql_server`.)
            _ => {
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

/// MySQL changes that restate the column, built from its current definition.
/// None when the definition isn't known, or the change doesn't restate.
fn mysql_restated(target: &str, change: &Change, current: &Current) -> Option<Vec<String>> {
    let engine = Engine::MySql;
    let column = |name: &str| current.definitions.get(name).map(|d| MySqlColumn::parse(d));
    let statement = match change {
        Change::RenameColumn { from, to, .. } => {
            let definition = column(from)?;
            format!(
                "ALTER TABLE {target} CHANGE COLUMN {} {} {};",
                quote::ident(engine, from),
                quote::ident(engine, if to.is_empty() { from } else { to }),
                definition.definition()
            )
        }
        Change::SetType { name, data_type } => {
            let mut definition = column(name)?;
            definition.set_type(data_type);
            modify(target, name, &definition, "")
        }
        Change::SetNullable { name, nullable, .. } => {
            let mut definition = column(name)?;
            definition.set_nullable(*nullable);
            modify(target, name, &definition, "")
        }
        Change::SetComment { name, comment, .. } => {
            let mut definition = column(name)?;
            definition.set_comment(comment);
            modify(target, name, &definition, "")
        }
        Change::MoveColumn { name, after } => {
            let definition = column(name)?;
            let position = match after {
                Some(previous) => format!(" AFTER {}", quote::ident(engine, previous)),
                None => " FIRST".to_owned(),
            };
            modify(target, name, &definition, &position)
        }
        _ => return None,
    };
    Some(vec![statement])
}

fn modify(target: &str, name: &str, definition: &MySqlColumn, position: &str) -> String {
    format!(
        "ALTER TABLE {target} MODIFY COLUMN {} {}{position};",
        quote::ident(Engine::MySql, name),
        definition.definition()
    )
}

/// SQL Server's own shapes for the changes it does differently: a rename is
/// a procedure call, a default is a named constraint, and a comment is an
/// extended property. None for the changes the shared code handles.
fn sql_server(
    schema: &str,
    table: &str,
    target: &str,
    change: &Change,
    current: &Current,
) -> Option<Vec<String>> {
    let engine = Engine::SqlServer;
    let schema = if schema.is_empty() { "dbo" } else { schema };
    let object = format!(
        "{}.{}",
        quote::ident(engine, schema),
        quote::ident(engine, table)
    );
    Some(match change {
        // sp_rename takes the old name as one string and the new name bare;
        // both are literals, so a quote in either can't end the string early.
        Change::RenameColumn { from, to, .. } => vec![format!(
            "EXEC sp_rename {}, {}, N'COLUMN';",
            quote::literal(engine, &format!("{object}.{}", quote::ident(engine, from))),
            quote::literal(engine, if to.is_empty() { from } else { to })
        )],
        // ALTER COLUMN restates nullability; left out, the column becomes
        // nullable. The default is a separate constraint and stays.
        Change::SetType { name, data_type } => {
            let nullability = match current.nullable.get(name) {
                Some(true) => " NULL",
                Some(false) => " NOT NULL",
                None => "",
            };
            vec![format!(
                "ALTER TABLE {target} ALTER COLUMN {} {}{nullability};",
                quote::ident(engine, name),
                data_type.trim(),
            )]
        }
        // A default is a constraint with a name: the old one is dropped by
        // that name before a new one is added.
        Change::SetDefault { name, default } => {
            let mut out = Vec::new();
            if let Some(existing) = current.default_constraints.get(name) {
                out.push(format!(
                    "ALTER TABLE {target} DROP CONSTRAINT {};",
                    quote::ident(engine, existing)
                ));
            }
            if !default.trim().is_empty() {
                let constraint: String = format!("DF_{table}_{name}").chars().take(128).collect();
                out.push(format!(
                    "ALTER TABLE {target} ADD CONSTRAINT {} DEFAULT {} FOR {};",
                    quote::ident(engine, &constraint),
                    default.trim(),
                    quote::ident(engine, name)
                ));
            }
            out
        }
        // A description is the MS_Description extended property: updated if
        // there is one, added if not, dropped when cleared.
        Change::SetComment { name, comment, .. } => {
            let levels = format!(
                "N'SCHEMA', {}, N'TABLE', {}, N'COLUMN', {}",
                quote::literal(engine, schema),
                quote::literal(engine, table),
                quote::literal(engine, name)
            );
            let exists = format!(
                "EXISTS (SELECT 1 FROM sys.extended_properties WHERE name = N'MS_Description' \
                 AND major_id = OBJECT_ID({}) AND minor_id = COLUMNPROPERTY(OBJECT_ID({}), {}, 'ColumnId'))",
                quote::literal(engine, &object),
                quote::literal(engine, &object),
                quote::literal(engine, name)
            );
            if comment.trim().is_empty() {
                vec![format!(
                    "IF {exists} EXEC sp_dropextendedproperty N'MS_Description', {levels};"
                )]
            } else {
                let value = quote::literal(engine, comment.trim());
                vec![format!(
                    "IF {exists} EXEC sp_updateextendedproperty N'MS_Description', {value}, {levels} \
                     ELSE EXEC sp_addextendedproperty N'MS_Description', {value}, {levels};"
                )]
            }
        }
        _ => return None,
    })
}

#[cfg(test)]
#[path = "tests/alter.rs"]
mod tests;

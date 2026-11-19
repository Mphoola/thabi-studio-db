//! Creating and dropping databases and schemas.
//!
//! These never run inside a transaction: PostgreSQL refuses CREATE DATABASE
//! in one, and none of the three can take a database back once it's dropped.
//! Every drop is confirmed by typing the name before it is offered.

use super::super::{Engine, quote};
use super::plan::DdlPreview;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ContainerAction {
    CreateDatabase {
        name: String,
    },
    DropDatabase {
        name: String,
    },
    /// A schema in `database` (PostgreSQL and SQL Server; on MySQL a schema
    /// is a database).
    CreateSchema {
        database: String,
        name: String,
    },
    DropSchema {
        database: String,
        name: String,
    },
}

impl ContainerAction {
    /// The database a statement has to run in, if it matters.
    pub fn runs_in(&self) -> Option<&str> {
        match self {
            Self::CreateSchema { database, .. } | Self::DropSchema { database, .. } => {
                Some(database)
            }
            _ => None,
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::CreateDatabase { name }
            | Self::DropDatabase { name }
            | Self::CreateSchema { name, .. }
            | Self::DropSchema { name, .. } => name,
        }
    }
}

/// The statements for a database or schema action, or why it can't be done.
pub fn container_script(engine: Engine, action: &ContainerAction) -> Result<Vec<String>, String> {
    let name = action.name().trim();
    if name.is_empty() {
        return Err("Give it a name.".to_owned());
    }
    let ident = quote::ident(engine, name);
    Ok(match action {
        ContainerAction::CreateDatabase { .. } => vec![format!("CREATE DATABASE {ident};")],
        ContainerAction::DropDatabase { .. } => vec![format!("DROP DATABASE {ident};")],
        ContainerAction::CreateSchema { database, .. }
        | ContainerAction::DropSchema { database, .. } => {
            let verb = if matches!(action, ContainerAction::CreateSchema { .. }) {
                "CREATE"
            } else {
                "DROP"
            };
            match engine {
                Engine::MySql => {
                    return Err(
                        "MySQL has no schemas inside a database; a schema there is a database."
                            .to_owned(),
                    );
                }
                // In the session's own database, which the command opens.
                Engine::Postgres => vec![format!("{verb} SCHEMA {ident};")],
                // CREATE SCHEMA must be alone in its batch, and in the right
                // database: run it through that database's sp_executesql.
                Engine::SqlServer => vec![format!(
                    "EXEC {}.sys.sp_executesql {};",
                    quote::ident(engine, database),
                    quote::literal(engine, &format!("{verb} SCHEMA {ident}"))
                )],
            }
        }
    })
}

/// What the review dialog shows for one.
pub fn container_preview(statements: Vec<String>, action: &ContainerAction) -> DdlPreview {
    let note = match action {
        ContainerAction::DropDatabase { .. } => {
            "Runs on its own, outside a transaction. Every table and row in the database is \
             removed, and this can't be undone."
        }
        ContainerAction::DropSchema { .. } => {
            "Runs on its own, outside a transaction. The server refuses to drop a schema that \
             still has objects in it."
        }
        _ => "Runs on its own, outside a transaction.",
    };
    DdlPreview {
        statements,
        note: note.to_owned(),
        transactional: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn databases_and_schemas_per_engine() {
        let create = ContainerAction::CreateDatabase {
            name: "sales 2026".into(),
        };
        assert_eq!(
            container_script(Engine::Postgres, &create).unwrap(),
            ["CREATE DATABASE \"sales 2026\";"]
        );
        assert_eq!(
            container_script(Engine::MySql, &create).unwrap(),
            ["CREATE DATABASE `sales 2026`;"]
        );
        let schema = ContainerAction::CreateSchema {
            database: "shop".into(),
            name: "it's".into(),
        };
        assert_eq!(
            container_script(Engine::SqlServer, &schema).unwrap(),
            ["EXEC [shop].sys.sp_executesql N'CREATE SCHEMA [it''s]';"]
        );
        assert!(container_script(Engine::MySql, &schema).is_err());
        assert!(
            container_script(
                Engine::Postgres,
                &ContainerAction::DropDatabase { name: " ".into() }
            )
            .is_err()
        );
    }

    #[test]
    fn the_webview_names_actions_by_kind() {
        let action: ContainerAction =
            serde_json::from_str(r#"{ "kind": "dropSchema", "database": "shop", "name": "old" }"#)
                .unwrap();
        assert_eq!(action.runs_in(), Some("shop"));
    }
}

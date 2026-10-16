//! The statements for a table's staged changes, in the order they must run.
//!
//! This is the promise the review dialog makes: the text it shows is produced
//! here, and the apply path produces it from the same call, so what is read
//! cannot drift from what is sent.

use super::super::Engine;
use super::alter::{Change, alter_column};
use super::keys::{IndexSpec, KeyChange, alter_key, create_index, drop_index};
use super::table::TableSpec;

/// An index to add or drop.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(rename_all_fields = "camelCase")]
pub enum IndexAction {
    Create(IndexSpec),
    Drop { name: String },
}

/// Everything staged against one table.
///
/// A plan either creates the table or changes one. Both go through the same
/// preview and apply so the review dialog is the same screen either way, but
/// they are not mixed: `ALTER TABLE … ADD COLUMN` against a table that is not
/// there yet would fail on the first statement, and a `CREATE TABLE` in the
/// middle of a plan of changes would be worse.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct TablePlan {
    /// Set when the table is being created rather than changed.
    #[serde(default)]
    pub create: Option<TableSpec>,
    #[serde(default)]
    pub columns: Vec<Change>,
    #[serde(default)]
    pub keys: Vec<KeyChange>,
    #[serde(default)]
    pub indexes: Vec<IndexAction>,
}

/// Whether a failure part way through leaves the table changed.
///
/// This is not the same on every engine, and the review dialog says which it
/// is rather than letting someone believe a rollback happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Atomicity {
    /// A transaction holds them together: a failure leaves the table as it was.
    Transactional,
    /// Each statement stands on its own, because DDL commits implicitly.
    PerStatement,
}

pub fn atomicity(engine: Engine) -> Atomicity {
    match engine {
        // PostgreSQL and SQL Server hold DDL in a transaction.
        Engine::MySql => Atomicity::PerStatement,
        _ => Atomicity::Transactional,
    }
}

/// The warning the review dialog shows about how the change will be applied.
pub fn atomicity_note(engine: Engine) -> &'static str {
    match atomicity(engine) {
        Atomicity::Transactional => {
            "These run in one transaction. If one fails, none of them are applied."
        }
        Atomicity::PerStatement => {
            "MySQL commits each statement on its own, so a failure part way through leaves \
             the statements before it applied. The rest are not run."
        }
    }
}

/// The full script for the staged changes, in an order that can succeed.
///
/// Keys and indexes come off before the columns they depend on, and go back on
/// afterwards: dropping a column that a foreign key still references is refused
/// by all three engines, and so is a column that an index still points at.
pub fn table_script(
    engine: Engine,
    database: &str,
    schema: &str,
    table: &str,
    plan: &TablePlan,
) -> Vec<String> {
    // A plan that creates the table is one statement, or an error: the
    // changes in a create plan are the columns it is created with.
    if let Some(spec) = &plan.create {
        return match super::table::create_table(engine, database, schema, table, spec) {
            Ok(statement) => vec![statement],
            Err(message) => vec![format!("-- Not run: {message}")],
        };
    }
    let target = (database, schema, table);
    let mut out = Vec::new();

    for change in &plan.keys {
        let removing = matches!(
            change,
            KeyChange::DropPrimaryKey { .. }
                | KeyChange::DropForeignKey { .. }
                | KeyChange::DropCheck { .. }
        );
        if removing {
            out.extend(alter_key(engine, target.0, target.1, target.2, change));
        }
    }
    for action in &plan.indexes {
        if let IndexAction::Drop { name } = action {
            out.push(drop_index(engine, target.0, target.1, target.2, name));
        }
    }
    for change in &plan.columns {
        // A rename that SQL Server does with sp_rename also restates the
        // column, and both are part of the one change.
        out.extend(alter_column(engine, target.0, target.1, target.2, change));
    }
    for change in &plan.keys {
        let removing = matches!(
            change,
            KeyChange::DropPrimaryKey { .. }
                | KeyChange::DropForeignKey { .. }
                | KeyChange::DropCheck { .. }
        );
        if !removing {
            out.extend(alter_key(engine, target.0, target.1, target.2, change));
        }
    }
    for action in &plan.indexes {
        if let IndexAction::Create(index) = action {
            out.push(create_index(engine, target.0, target.1, target.2, index));
        }
    }
    out
}

/// The statements for a new table, which is one statement unless it has keys
/// the engine wants restated.
pub fn create_script(
    engine: Engine,
    database: &str,
    schema: &str,
    name: &str,
    spec: &TableSpec,
) -> Result<Vec<String>, String> {
    Ok(vec![super::table::create_table(
        engine, database, schema, name, spec,
    )?])
}

#[cfg(test)]
mod tests {
    use super::super::table::{ColumnSpec, ForeignKey};
    use super::*;

    #[test]
    fn constraints_come_off_before_the_columns_and_go_back_on_after() {
        let plan = TablePlan {
            create: None,
            columns: vec![Change::DropColumn {
                name: "note".into(),
            }],
            keys: vec![
                // Adding a key that depends on the column being added.
                KeyChange::DropForeignKey {
                    name: "orders_customer_fkey".into(),
                },
                KeyChange::AddForeignKey(ForeignKey {
                    name: "orders_customer_fkey".into(),
                    columns: vec!["customer_id".into()],
                    references: vec!["public".into(), "customers".into()],
                    referenced_columns: vec!["id".into()],
                    on_delete: "CASCADE".into(),
                    on_update: String::new(),
                }),
            ],
            indexes: vec![
                IndexAction::Drop {
                    name: "orders_note_idx".into(),
                },
                IndexAction::Create(IndexSpec {
                    name: "orders_status_idx".into(),
                    columns: vec!["status".into()],
                    unique: true,
                }),
            ],
        };
        let script = table_script(Engine::Postgres, "shop", "public", "orders", &plan);
        let at = |needle: &str| {
            script
                .iter()
                .position(|sql| sql.contains(needle))
                .unwrap_or_else(|| panic!("no statement for {needle}"))
        };
        // Dropped before the column it referenced, added after it exists again.
        assert!(at("DROP CONSTRAINT") < at("DROP COLUMN"));
        assert!(at("DROP INDEX") < at("DROP COLUMN"));
        assert!(at("DROP COLUMN") < at("ADD CONSTRAINT"));
        assert!(at("DROP COLUMN") < at("CREATE UNIQUE INDEX"));
        assert_eq!(script.len(), 5);
    }

    #[test]
    fn a_sql_server_rename_keeps_its_two_statements_together() {
        let plan = TablePlan {
            create: None,
            columns: vec![Change::RenameColumn {
                from: "note".into(),
                to: "summary".into(),
                spec: ColumnSpec {
                    name: "summary".into(),
                    data_type: "TEXT".into(),
                    nullable: true,
                    ..Default::default()
                },
            }],
            ..TablePlan::default()
        };
        let script = table_script(Engine::SqlServer, "shop", "dbo", "orders", &plan);
        assert_eq!(script.len(), 2);
        assert!(script[0].contains("sp_rename"));
        assert!(script[1].contains("ALTER COLUMN"));
    }

    #[test]
    fn a_create_plan_is_one_statement_not_a_list_of_alters() {
        // ALTER TABLE ADD COLUMN against a table that is not there would fail
        // on the first statement, which is the whole difference.
        let plan = TablePlan {
            create: Some(TableSpec {
                columns: vec![ColumnSpec {
                    name: "id".into(),
                    data_type: "INTEGER".into(),
                    nullable: false,
                    primary_key: true,
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..TablePlan::default()
        };
        for engine in [Engine::Postgres, Engine::MySql, Engine::SqlServer] {
            let script = table_script(engine, "shop", "public", "orders", &plan);
            assert_eq!(script.len(), 1, "{engine:?}: {script:?}");
            assert!(script[0].starts_with("CREATE TABLE"), "{engine:?}");
        }
    }

    #[test]
    fn a_create_plan_with_no_columns_says_so_rather_than_running_nothing() {
        let plan = TablePlan {
            create: Some(TableSpec::default()),
            ..TablePlan::default()
        };
        let script = table_script(Engine::Postgres, "shop", "public", "orders", &plan);
        assert_eq!(script.len(), 1);
        assert!(script[0].starts_with("-- Not run:"), "{}", script[0]);
    }

    #[test]
    fn an_empty_plan_is_an_empty_script() {
        let plan = TablePlan::default();
        assert!(table_script(Engine::Postgres, "shop", "public", "orders", &plan).is_empty());
    }

    #[test]
    fn every_statement_of_a_plan_stands_alone() {
        let plan = TablePlan {
            create: None,
            columns: vec![Change::AddColumn(ColumnSpec {
                name: "note".into(),
                data_type: "TEXT".into(),
                nullable: true,
                ..Default::default()
            })],
            keys: vec![KeyChange::AddCheck {
                name: "note_not_blank".into(),
                expression: "note <> ''".into(),
            }],
            indexes: vec![IndexAction::Create(IndexSpec {
                name: "orders_note_idx".into(),
                columns: vec!["note".into()],
                unique: false,
            })],
        };
        for engine in [Engine::Postgres, Engine::MySql, Engine::SqlServer] {
            for statement in table_script(engine, "shop", "public", "orders", &plan) {
                assert!(statement.ends_with(';'), "{engine:?}: {statement}");
                assert!(
                    !statement.trim_end().ends_with(";;"),
                    "{engine:?} doubled the terminator: {statement}"
                );
            }
        }
    }

    #[test]
    fn the_dialog_is_told_which_way_it_can_fail() {
        // MySQL's DDL commits implicitly, so a rollback there is a hope rather
        // than a guarantee, and the dialog must not imply otherwise.
        assert_eq!(atomicity(Engine::MySql), Atomicity::PerStatement);
        assert_eq!(atomicity(Engine::Postgres), Atomicity::Transactional);
        assert_eq!(atomicity(Engine::SqlServer), Atomicity::Transactional);
        assert!(atomicity_note(Engine::MySql).contains("commits each statement"));
        assert!(atomicity_note(Engine::Postgres).contains("one transaction"));
    }
}
/// The table a change is against, as the webview names it.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableTarget {
    pub database: String,
    #[serde(default)]
    pub schema: Option<String>,
    pub name: String,
}

/// What the review dialog shows: the statements, and what happens if one fails.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DdlPreview {
    /// The statements, in the order they will run.
    pub statements: Vec<String>,
    /// Shown under them, because it differs by engine.
    pub note: String,
    /// True when a failure leaves the table as it was.
    pub transactional: bool,
}

/// Builds the preview. The same call builds the script that is applied.
pub fn preview(
    engine: Engine,
    database: &str,
    schema: &str,
    table: &str,
    plan: &TablePlan,
) -> DdlPreview {
    DdlPreview {
        statements: table_script(engine, database, schema, table, plan),
        note: atomicity_note(engine).to_owned(),
        transactional: atomicity(engine) == Atomicity::Transactional,
    }
}

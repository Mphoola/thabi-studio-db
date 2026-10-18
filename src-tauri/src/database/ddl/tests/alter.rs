use super::*;
use crate::database::ddl::table_script;

const PG: (&str, &str) = ("shop", "public");
const MY: (&str, &str) = ("shop", "");
const MS: (&str, &str) = ("shop", "dbo");

fn column(name: &str, data_type: &str) -> ColumnSpec {
    ColumnSpec {
        name: name.into(),
        data_type: data_type.into(),
        nullable: true,
        ..ColumnSpec::default()
    }
}

#[test]
fn adds_and_drops_a_column() {
    let add = ColumnSpec {
        nullable: false,
        default: "'x'".into(),
        ..column("note", "TEXT")
    };
    assert_eq!(
        alter_column(
            Engine::Postgres,
            PG.0,
            PG.1,
            "orders",
            &Change::AddColumn(add.clone())
        ),
        ["ALTER TABLE \"public\".\"orders\" ADD COLUMN \"note\" TEXT NOT NULL DEFAULT 'x';"]
    );
    assert_eq!(
        alter_column(Engine::MySql, MY.0, MY.1, "orders", &Change::AddColumn(add)),
        ["ALTER TABLE `shop`.`orders` ADD COLUMN `note` TEXT NOT NULL DEFAULT 'x';"]
    );
    assert_eq!(
        alter_column(
            Engine::SqlServer,
            MS.0,
            MS.1,
            "orders",
            &Change::DropColumn {
                name: "note".into()
            }
        ),
        ["ALTER TABLE [shop].[dbo].[orders] DROP COLUMN [note];"]
    );
}

#[test]
fn renames_a_column_the_way_each_engine_does() {
    let spec = column("summary", "TEXT");
    let rename = |spec: ColumnSpec| Change::RenameColumn {
        from: "note".into(),
        to: "summary".into(),
        spec,
    };
    // MySQL renames by restating the column.
    assert_eq!(
        alter_column(Engine::MySql, MY.0, MY.1, "orders", &rename(spec.clone())),
        ["ALTER TABLE `shop`.`orders` CHANGE COLUMN `note` `summary` TEXT;"]
    );
    assert_eq!(
        alter_column(
            Engine::Postgres,
            PG.0,
            PG.1,
            "orders",
            &rename(spec.clone())
        ),
        ["ALTER TABLE \"public\".\"orders\" RENAME COLUMN \"note\" TO \"summary\";"]
    );
    // SQL Server goes through a procedure, and then still needs the type
    // restated because sp_rename does not touch it.
    let statements = alter_column(Engine::SqlServer, MS.0, MS.1, "orders", &rename(spec));
    assert_eq!(
        statements.len(),
        2,
        "one statement cannot rename and retype"
    );
    assert!(
        statements[0].contains("sp_rename N'[shop].[dbo].[orders].[note]', N'summary', N'COLUMN';")
    );
    assert!(statements[1].starts_with("ALTER TABLE [shop].[dbo].[orders] ALTER COLUMN [summary]"));
}

#[test]
fn changing_a_type_says_how_on_postgres() {
    // Postgres will not cast implicitly, and a cast that cannot work is
    // the server's to refuse rather than ours to guess.
    assert_eq!(
        alter_column(
            Engine::Postgres,
            PG.0,
            PG.1,
            "orders",
            &Change::SetType {
                name: "total".into(),
                data_type: "NUMERIC(10,2)".into()
            }
        ),
        [
            "ALTER TABLE \"public\".\"orders\" ALTER COLUMN \"total\" TYPE NUMERIC(10,2) USING \"total\"::NUMERIC(10,2);"
        ]
    );
    assert_eq!(
        alter_column(
            Engine::MySql,
            MY.0,
            MY.1,
            "orders",
            &Change::SetType {
                name: "total".into(),
                data_type: "DECIMAL(10,2)".into()
            }
        ),
        // MySQL spells a restated column MODIFY. ALTER COLUMN is PostgreSQL's
        // syntax and MySQL refuses it — found by running it, not by reading it.
        ["ALTER TABLE `shop`.`orders` MODIFY COLUMN `total` DECIMAL(10,2);"]
    );
    // SQL Server happens to use ALTER COLUMN, which is why the two must not
    // be lumped together as "not PostgreSQL".
    assert_eq!(
        alter_column(
            Engine::SqlServer,
            MS.0,
            MS.1,
            "orders",
            &Change::SetType {
                name: "total".into(),
                data_type: "DECIMAL(10,2)".into()
            }
        ),
        ["ALTER TABLE [shop].[dbo].[orders] ALTER COLUMN [total] DECIMAL(10,2);"]
    );
}

#[test]
fn nullability_restates_the_column_where_it_has_to() {
    let change = Change::SetNullable {
        name: "note".into(),
        nullable: false,
        data_type: "TEXT".into(),
    };
    assert_eq!(
        alter_column(Engine::Postgres, PG.0, PG.1, "orders", &change),
        ["ALTER TABLE \"public\".\"orders\" ALTER COLUMN \"note\" SET NOT NULL;"]
    );
    // MySQL has no SET NOT NULL; it is part of the column.
    assert_eq!(
        alter_column(Engine::MySql, MY.0, MY.1, "orders", &change),
        ["ALTER TABLE `shop`.`orders` MODIFY COLUMN `note` TEXT NOT NULL;"]
    );
    assert_eq!(
        alter_column(Engine::SqlServer, MS.0, MS.1, "orders", &change),
        ["ALTER TABLE [shop].[dbo].[orders] ALTER COLUMN [note] TEXT NOT NULL;"]
    );
}

#[test]
fn sets_and_drops_a_default() {
    let set = Change::SetDefault {
        name: "status".into(),
        default: "'new'".into(),
    };
    assert_eq!(
        alter_column(Engine::Postgres, PG.0, PG.1, "orders", &set),
        ["ALTER TABLE \"public\".\"orders\" ALTER COLUMN \"status\" SET DEFAULT 'new';"]
    );
    assert_eq!(
        alter_column(Engine::SqlServer, MS.0, MS.1, "orders", &set),
        ["ALTER TABLE [shop].[dbo].[orders] ALTER COLUMN [status] SET DEFAULT 'new';"]
    );
    assert_eq!(
        alter_column(
            Engine::Postgres,
            PG.0,
            PG.1,
            "orders",
            &Change::SetDefault {
                name: "status".into(),
                default: "  ".into()
            }
        ),
        ["ALTER TABLE \"public\".\"orders\" ALTER COLUMN \"status\" DROP DEFAULT;"]
    );
}

#[test]
fn a_comment_is_where_each_engine_keeps_one() {
    let change = Change::SetComment {
        name: "status".into(),
        comment: "where it is".into(),
        data_type: "VARCHAR(20)".into(),
    };
    assert_eq!(
        alter_column(Engine::MySql, MY.0, MY.1, "orders", &change),
        ["ALTER TABLE `shop`.`orders` MODIFY COLUMN `status` VARCHAR(20) COMMENT 'where it is';"]
    );
    // A comment is its own statement on Postgres, and a bare keyword
    // rather than a string when there is nothing to say.
    assert_eq!(
        alter_column(Engine::Postgres, PG.0, PG.1, "orders", &change),
        ["COMMENT ON COLUMN \"public\".\"orders\".\"status\" IS 'where it is';"]
    );
    assert_eq!(
        alter_column(
            Engine::Postgres,
            PG.0,
            PG.1,
            "orders",
            &Change::SetComment {
                name: "status".into(),
                comment: "".into(),
                data_type: "VARCHAR(20)".into()
            }
        ),
        ["COMMENT ON COLUMN \"public\".\"orders\".\"status\" IS NULL;"]
    );
    assert!(
        alter_column(Engine::SqlServer, MS.0, MS.1, "orders", &change)[0]
            .contains("sp_updateextendedproperty")
    );
}

#[test]
fn moves_a_column_only_where_column_order_means_something() {
    let change = Change::MoveColumn {
        name: "note".into(),
        after: Some("id".into()),
    };
    assert_eq!(
        alter_column(Engine::MySql, MY.0, MY.1, "orders", &change),
        ["ALTER TABLE `shop`.`orders` MODIFY COLUMN `note` AFTER `id`;"]
    );
    // Nothing to run is better than a statement the server would refuse.
    assert!(alter_column(Engine::Postgres, PG.0, PG.1, "orders", &change).is_empty());
    assert!(alter_column(Engine::SqlServer, MS.0, MS.1, "orders", &change).is_empty());
    assert_eq!(
        alter_column(
            Engine::MySql,
            MY.0,
            MY.1,
            "orders",
            &Change::MoveColumn {
                name: "note".into(),
                after: None
            }
        ),
        ["ALTER TABLE `shop`.`orders` MODIFY COLUMN `note` FIRST;"]
    );
}

/// The review dialog's promise: the text it shows is the text that runs.
/// Both come from `table_script`, and this checks the preview is that script
/// and that the engine's own failure behaviour is stated rather than implied.
#[test]
fn the_preview_is_the_script_that_will_run() {
    use crate::database::ddl::{
        Change, ColumnSpec, ForeignKey, IndexAction, IndexSpec, KeyChange, TablePlan, preview,
    };

    let plan = TablePlan {
        create: None,
        columns: vec![
            Change::AddColumn(ColumnSpec {
                name: "summary".into(),
                data_type: "TEXT".into(),
                nullable: true,
                ..Default::default()
            }),
            Change::DropColumn {
                name: "note".into(),
            },
        ],
        keys: vec![
            KeyChange::DropForeignKey {
                name: "orders_note_fkey".into(),
            },
            KeyChange::AddForeignKey(ForeignKey {
                name: "orders_summary_fkey".into(),
                columns: vec!["summary_id".into()],
                references: vec!["public".into(), "summaries".into()],
                referenced_columns: vec!["id".into()],
                on_delete: "SET NULL".into(),
                on_update: String::new(),
            }),
        ],
        indexes: vec![IndexAction::Create(IndexSpec {
            name: "orders_summary_idx".into(),
            columns: vec!["summary".into()],
            unique: true,
        })],
    };

    for engine in [Engine::Postgres, Engine::MySql, Engine::SqlServer] {
        let shown = preview(engine, "shop", "public", "orders", &plan);
        let ran = table_script(engine, "shop", "public", "orders", &plan);
        assert_eq!(
            shown.statements, ran,
            "{engine:?}: preview drifted from apply"
        );
        assert!(!shown.statements.is_empty());
        assert!(shown.note.contains("transaction") || shown.note.contains("commits"));
        // The claim matches the engine.
        assert_eq!(shown.transactional, engine != Engine::MySql, "{engine:?}");
    }
}

/// A statement that would fail part way through has to be honest about the
/// half that will already be applied.
#[test]
fn mysql_says_it_cannot_roll_back() {
    let my =
        crate::database::ddl::preview(Engine::MySql, "shop", "", "orders", &Default::default());
    assert!(!my.transactional);
    assert!(my.note.contains("commits each statement"));
}

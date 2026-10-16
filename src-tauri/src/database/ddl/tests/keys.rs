use crate::database::ddl::keys::*;
use crate::database::ddl::outside_identifiers;
use crate::database::ddl::{Change, alter_column};

/// The database and schema each engine addresses a table from.
const PG: (&str, &str) = ("shop", "public");
const MY: (&str, &str) = ("shop", "");
const MS: (&str, &str) = ("shop", "dbo");

#[test]
fn indexes_live_in_different_places() {
    let index = IndexSpec {
        name: "orders_status_idx".into(),
        columns: vec!["status".into(), "created_at".into()],
        unique: false,
    };
    assert_eq!(
        create_index(Engine::Postgres, PG.0, PG.1, "orders", &index),
        "CREATE INDEX \"orders_status_idx\" ON \"public\".\"orders\" (\"status\", \"created_at\");"
    );
    // MySQL keeps an index inside the table's own statement.
    assert_eq!(
        create_index(Engine::MySql, MY.0, MY.1, "orders", &index),
        "ALTER TABLE `shop`.`orders` ADD INDEX `orders_status_idx` (`status`, `created_at`);"
    );
    assert_eq!(
        create_index(
            Engine::Postgres,
            PG.0,
            PG.1,
            "orders",
            &IndexSpec {
                unique: true,
                ..index.clone()
            }
        ),
        "CREATE UNIQUE INDEX \"orders_status_idx\" ON \"public\".\"orders\" (\"status\", \"created_at\");"
    );
    assert_eq!(
        drop_index(Engine::Postgres, PG.0, PG.1, "orders", "orders_status_idx"),
        "DROP INDEX \"orders_status_idx\";"
    );
    assert_eq!(
        drop_index(Engine::MySql, MY.0, MY.1, "orders", "orders_status_idx"),
        "ALTER TABLE `shop`.`orders` DROP INDEX `orders_status_idx`;"
    );
}

#[test]
fn keys_are_dropped_by_the_name_they_were_given() {
    // Guessing a name would produce "constraint does not exist" from the
    // server, which reads as the app being broken rather than the name.
    assert_eq!(
        alter_key(
            Engine::Postgres,
            PG.0,
            PG.1,
            "orders",
            &KeyChange::DropPrimaryKey {
                name: "orders_pkey".into()
            }
        ),
        ["ALTER TABLE \"public\".\"orders\" DROP CONSTRAINT \"orders_pkey\";"]
    );
    assert_eq!(
        alter_key(
            Engine::SqlServer,
            MS.0,
            MS.1,
            "orders",
            &KeyChange::DropForeignKey {
                name: "orders_customer_fkey".into()
            }
        ),
        ["ALTER TABLE [shop].[dbo].[orders] DROP CONSTRAINT [orders_customer_fkey];"]
    );
}

#[test]
fn adds_a_key_and_a_check() {
    assert_eq!(
        alter_key(
            Engine::Postgres,
            PG.0,
            PG.1,
            "orders",
            &KeyChange::SetPrimaryKey {
                name: String::new(),
                columns: vec!["tenant_id".into(), "id".into()]
            }
        ),
        ["ALTER TABLE \"public\".\"orders\" ADD PRIMARY KEY (\"tenant_id\", \"id\");"]
    );
    assert_eq!(
        alter_key(
            Engine::SqlServer,
            MS.0,
            MS.1,
            "orders",
            &KeyChange::AddCheck {
                name: "total_positive".into(),
                expression: "total >= 0".into()
            }
        ),
        ["ALTER TABLE [shop].[dbo].[orders] ADD CONSTRAINT [total_positive] CHECK (total >= 0);"]
    );
}

#[test]
fn every_generated_statement_quotes_an_awkward_name() {
    let awkward = "order\"; DROP TABLE x; --";
    let statements = [
        alter_column(
            Engine::Postgres,
            PG.0,
            PG.1,
            "orders",
            &Change::SetType {
                name: awkward.into(),
                data_type: "TEXT".into(),
            },
        ),
        alter_column(
            Engine::MySql,
            MY.0,
            MY.1,
            "orders",
            &Change::DropColumn {
                name: awkward.into(),
            },
        ),
        vec![create_index(
            Engine::Postgres,
            PG.0,
            PG.1,
            "orders",
            &IndexSpec {
                name: awkward.into(),
                columns: vec!["id".into()],
                unique: true,
            },
        )],
        alter_key(
            Engine::SqlServer,
            MS.0,
            MS.1,
            "orders",
            &KeyChange::DropCheck {
                name: awkward.into(),
            },
        ),
    ]
    .concat();
    for statement in statements {
        // Exactly one statement per call, whatever the name contained.
        let outside = outside_identifiers(&statement);
        assert_eq!(
            outside.matches(';').count(),
            1,
            "a name ended the statement: {statement}"
        );
    }
}

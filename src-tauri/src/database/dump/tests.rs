//! Tests for the SQL dump.
//!
//! Out of dump.rs because the module is at its size limit and a test is not
//! part of what it does.

use super::*;
use crate::database::metadata::{ColumnInfo, ColumnRef, IndexInfo};
use crate::database::values::Cell;

fn structure() -> ObjectStructure {
    ObjectStructure {
        columns: vec![super::super::metadata::ColumnInfo {
            name: "id".into(),
            data_type: "INTEGER".into(),
            nullable: false,
            default: None,
            primary_key: Some(1),
            enum_values: None,
            references: None,
        }],
        indexes: Some(vec![IndexInfo {
            name: "PRIMARY".into(),
            columns: vec!["id".into()],
            unique: true,
            primary: true,
        }]),
        show_create: None,
    }
}

fn value(text: &str) -> Cell {
    Cell::Value(text.to_owned())
}

#[test]
fn inserts_name_their_columns() {
    let sql = insert_statement(
        Engine::Postgres,
        "\"public\".\"orders\"",
        &["id".into(), "name".into()],
        &[vec![value("1"), value("Ada")]],
    )
    .expect("insert");
    assert_eq!(
        sql,
        "INSERT INTO \"public\".\"orders\" (\"id\", \"name\") VALUES ('1', 'Ada');\n"
    );
}

#[test]
fn puts_several_rows_in_one_statement() {
    let sql = insert_statement(
        Engine::Postgres,
        "\"t\"",
        &["id".into()],
        &[vec![value("1")], vec![value("2")]],
    )
    .expect("insert");
    assert_eq!(sql, "INSERT INTO \"t\" (\"id\") VALUES ('1'), ('2');\n");
}

/// The escaping is `transfer::literal`'s, and the point of the test is that
/// a dump and a CSV of the same table cannot disagree about a value.
#[test]
fn quotes_values_the_way_the_engine_does() {
    let nasty = vec![vec![value("O'Hara"), Cell::Null]];
    let postgres =
        insert_statement(Engine::Postgres, "\"t\"", &["a".into(), "b".into()], &nasty).expect("pg");
    assert!(postgres.contains("'O''Hara'"), "{postgres}");
    assert!(postgres.contains("NULL"), "{postgres}");

    let mysql =
        insert_statement(Engine::MySql, "`t`", &["a".into(), "b".into()], &nasty).expect("my");
    assert!(mysql.contains("'O''Hara'"), "{mysql}");

    let mssql =
        insert_statement(Engine::SqlServer, "[t]", &["a".into(), "b".into()], &nasty).expect("s");
    // SQL Server needs N'' for a non-ASCII literal to survive.
    assert!(mssql.contains("N'O''Hara'"), "{mssql}");
}

#[test]
fn refuses_a_row_that_does_not_match_the_header() {
    let error = insert_statement(
        Engine::Postgres,
        "\"t\"",
        &["a".into(), "b".into()],
        &[vec![value("1")]],
    )
    .expect_err("shape");
    assert_eq!(error.kind, "row_shape_mismatch");
}

#[test]
fn writes_nothing_for_no_rows() {
    assert_eq!(
        insert_statement(Engine::Postgres, "\"t\"", &["a".into()], &[]).expect("empty"),
        ""
    );
}

#[test]
fn drops_before_it_creates() {
    assert_eq!(
        drop_table("`shop`.`orders`"),
        "DROP TABLE IF EXISTS `shop`.`orders`;\n"
    );
}

/// MySQL's own `SHOW CREATE TABLE` must not be rebuilt: it is the only
/// version of the statement that is guaranteed to recreate the table.
#[test]
fn mysql_uses_what_the_server_said() {
    let mut described = structure();
    described.show_create =
        Some("CREATE TABLE `orders` (\n  `id` int NOT NULL\n) ENGINE=InnoDB".to_owned());
    let statements = structure_statements(Engine::MySql, "shop", "", "orders", &described);
    assert!(
        statements[0].starts_with("DROP TABLE IF EXISTS"),
        "{statements:?}"
    );
    assert!(statements[1].contains("ENGINE=InnoDB"), "{statements:?}");
    // A rebuild would qualify the table and omit ENGINE; the server's own
    // words do neither.
    assert!(!statements[1].contains("`shop`.`orders`"), "{statements:?}");
}

#[test]
fn rebuilds_the_structure_where_there_is_no_show_create() {
    let statements =
        structure_statements(Engine::Postgres, "shop", "public", "orders", &structure());
    assert!(
        statements.iter().any(|s| s.contains("CREATE TABLE")),
        "{statements:?}"
    );
}

/// A table that cannot be described must leave a note, not a gap.
#[test]
fn says_so_when_it_cannot_write_the_structure() {
    let mut empty = structure();
    empty.columns.clear();
    let statements = structure_statements(Engine::Postgres, "shop", "public", "orders", &empty);
    assert!(
        statements.iter().any(|s| s.contains("Could not write")),
        "{statements:?}"
    );
}

#[test]
fn the_header_says_what_is_in_the_file() {
    let text = header(Engine::Postgres, "shop", "public", "2026-10-17 09:00");
    assert!(text.contains("PostgreSQL"), "{text}");
    assert!(text.contains("shop"), "{text}");
    assert!(text.contains("public"), "{text}");
    assert!(text.contains("2026-10-17 09:00"), "{text}");
    assert!(text.starts_with("--"), "{text}");
}

#[test]
fn says_whether_there_is_data_in_the_file() {
    let both = DumpOptions {
        structure: true,
        data: true,
        ..Default::default()
    };
    assert!(contents_note(&both).contains("structure and data"));
    let structure = DumpOptions {
        structure: true,
        ..Default::default()
    };
    assert!(contents_note(&structure).contains("structure only"));
    let data = DumpOptions {
        data: true,
        ..Default::default()
    };
    assert!(contents_note(&data).contains("data only"));
}

/// A dump that is a reconstruction must say so, or it is trusted as one.
#[test]
fn warns_where_the_structure_was_rebuilt_rather_than_quoted() {
    let options = DumpOptions {
        structure: true,
        data: true,
        ..Default::default()
    };
    let postgres = fidelity_note(Engine::Postgres, &options).expect("pg note");
    assert!(postgres.contains("rebuilt from the catalog"), "{postgres}");
    let mssql = fidelity_note(Engine::SqlServer, &options).expect("s note");
    assert!(mssql.contains("SQL Server"), "{mssql}");
}

#[test]
fn no_note_where_the_servers_own_words_are_used() {
    let options = DumpOptions {
        structure: true,
        ..Default::default()
    };
    assert!(fidelity_note(Engine::MySql, &options).is_none());
    // A data-only dump carries no structure to be a reconstruction of.
    let data = DumpOptions {
        data: true,
        ..Default::default()
    };
    assert!(fidelity_note(Engine::Postgres, &data).is_none());
}

#[test]
fn options_that_would_produce_an_empty_file_are_not_wanted() {
    assert!(!DumpOptions::default().wants_anything());
    assert!(
        DumpOptions {
            structure: true,
            ..Default::default()
        }
        .wants_anything()
    );
    assert!(
        DumpOptions {
            data: true,
            ..Default::default()
        }
        .wants_anything()
    );
}

#[test]
fn counts_what_it_wrote() {
    let mut writer = CountingWriter::new(Vec::new());
    writer.write_all(b"hello ").expect("write");
    writer.write_all(b"world").expect("write");
    assert_eq!(writer.bytes(), 11);
    assert_eq!(writer.into_inner(), b"hello world");
}

fn table_named(name: &str) -> (String, ObjectStructure) {
    (
        name.to_owned(),
        ObjectStructure {
            columns: vec![ColumnInfo {
                name: "id".into(),
                data_type: "INTEGER".into(),
                nullable: false,
                default: None,
                primary_key: Some(1),
                enum_values: None,
                references: None,
            }],
            indexes: None,
            show_create: None,
        },
    )
}

fn referencing(name: &str, target: &str) -> (String, ObjectStructure) {
    let (mut table_name, mut structure) = table_named(name);
    structure.columns.push(ColumnInfo {
        name: format!("{}_id", target),
        data_type: "INTEGER".into(),
        nullable: true,
        default: None,
        primary_key: None,
        enum_values: None,
        references: Some(ColumnRef {
            schema: "public".into(),
            table: target.into(),
            column: "id".into(),
        }),
    });
    table_name.truncate(table_name.len());
    (table_name, structure)
}

/// Alphabetical order put `orders` before the `customers` it referenced,
/// and the file failed on its first foreign key. Printing a real dump is
/// what showed it.
#[test]
fn a_referenced_table_is_created_before_the_one_that_references_it() {
    let ordered = order_for_restore(vec![
        referencing("orders", "customers"),
        table_named("customers"),
    ]);
    let names: Vec<&str> = ordered.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["customers", "orders"]);
}

#[test]
fn chains_are_created_from_the_far_end_back() {
    let ordered = order_for_restore(vec![
        referencing("c", "b"),
        referencing("b", "a"),
        table_named("a"),
    ]);
    let names: Vec<&str> = ordered.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["a", "b", "c"]);
}

/// The database allowed the cycle, so the dump cannot invent an order for
/// it. It must not loop, and it must not drop a table.
#[test]
fn tables_that_reference_each_other_still_all_appear() {
    let ordered = order_for_restore(vec![referencing("a", "b"), referencing("b", "a")]);
    assert_eq!(ordered.len(), 2);
}

#[test]
fn a_schema_with_no_keys_keeps_its_order() {
    let ordered = order_for_restore(vec![table_named("zebra"), table_named("apple")]);
    let names: Vec<&str> = ordered.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["zebra", "apple"]);
}

/// `create_table` already ends in a semicolon; adding another left `);;`
/// in every file.
#[test]
fn statements_end_in_exactly_one_semicolon() {
    let statements =
        structure_statements(Engine::Postgres, "shop", "public", "orders", &structure());
    for statement in &statements {
        let trimmed = statement.trim_end();
        assert!(!trimmed.ends_with(";;"), "double semicolon: {statement:?}");
        if trimmed.contains("CREATE") || trimmed.contains("INDEX") {
            assert!(trimmed.ends_with(';'), "no semicolon: {statement:?}");
        }
    }
}

//! Staged row edits on a table: the UPDATE, DELETE and INSERT statements the
//! user reviews and then applies in one transaction (see
//! `QueryTab::apply`). Rows are found by their primary key. Values are the
//! text the grid shows, written as literals the server converts to each
//! column's type; the user sees every statement before it runs.

use serde::{Deserialize, Serialize};

use super::{Engine, metadata::ColumnRef, quote};
use crate::errors::AppError;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnValue {
    pub column: String,
    /// None is SQL NULL.
    pub value: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RowChange {
    Update {
        key: Vec<ColumnValue>,
        set: Vec<ColumnValue>,
    },
    Delete {
        key: Vec<ColumnValue>,
    },
    Insert {
        values: Vec<ColumnValue>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableTarget {
    pub database: String,
    #[serde(default)]
    pub schema: String,
    pub name: String,
}

fn invalid(message: &str) -> AppError {
    AppError::new("invalid_edit", message.to_owned())
}

fn value(engine: Engine, value: &Option<String>) -> String {
    value
        .as_deref()
        .map_or_else(|| "NULL".to_owned(), |v| quote::literal(engine, v))
}

fn key_clause(engine: Engine, key: &[ColumnValue]) -> Result<String, AppError> {
    if key.is_empty() {
        return Err(invalid(
            "This table has no primary key, so its rows can't be edited here.",
        ));
    }
    key.iter()
        .map(|part| match &part.value {
            Some(v) => Ok(format!(
                "{} = {}",
                quote::ident(engine, &part.column),
                quote::literal(engine, v)
            )),
            None => Err(invalid("A row with a NULL key can't be edited.")),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join(" AND "))
}

/// The statements for `changes`, in the order they run: deletes, then
/// updates, then inserts, so a key that is deleted and added again works.
pub fn statements(
    engine: Engine,
    target: &TableTarget,
    changes: &[RowChange],
) -> Result<Vec<String>, AppError> {
    let table = quote::table(engine, &target.database, &target.schema, &target.name);
    let rank = |change: &RowChange| match change {
        RowChange::Delete { .. } => 0,
        RowChange::Update { .. } => 1,
        RowChange::Insert { .. } => 2,
    };
    let mut ordered: Vec<_> = changes.iter().collect();
    ordered.sort_by_key(|change| rank(change));
    ordered
        .into_iter()
        .map(|change| match change {
            RowChange::Delete { key } => Ok(format!(
                "DELETE FROM {table} WHERE {}",
                key_clause(engine, key)?
            )),
            RowChange::Update { key, set } => {
                if set.is_empty() {
                    return Err(invalid("An edited row has no changed columns."));
                }
                let assignments = set
                    .iter()
                    .map(|c| {
                        format!(
                            "{} = {}",
                            quote::ident(engine, &c.column),
                            value(engine, &c.value)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                Ok(format!(
                    "UPDATE {table} SET {assignments} WHERE {}",
                    key_clause(engine, key)?
                ))
            }
            RowChange::Insert { values } if values.is_empty() => Ok(match engine {
                Engine::MySql => format!("INSERT INTO {table} () VALUES ()"),
                _ => format!("INSERT INTO {table} DEFAULT VALUES"),
            }),
            RowChange::Insert { values } => {
                let columns = values
                    .iter()
                    .map(|c| quote::ident(engine, &c.column))
                    .collect::<Vec<_>>()
                    .join(", ");
                let literals = values
                    .iter()
                    .map(|c| value(engine, &c.value))
                    .collect::<Vec<_>>()
                    .join(", ");
                Ok(format!(
                    "INSERT INTO {table} ({columns}) VALUES ({literals})"
                ))
            }
        })
        .collect()
}

/// Rows offered when picking a foreign key value.
pub const LOOKUP_LIMIT: usize = 50;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LookupRow {
    pub value: String,
    /// A readable column of the referenced row (its name or title), if any.
    pub label: Option<String>,
}

/// A column of the referenced table that describes a row better than its
/// key: name, title, label… or anything ending in "name".
pub fn label_column<'a>(key: &str, columns: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    const PREFERRED: [&str; 8] = [
        "name",
        "title",
        "label",
        "display_name",
        "full_name",
        "description",
        "code",
        "email",
    ];
    let columns: Vec<_> = columns.into_iter().filter(|c| *c != key).collect();
    PREFERRED
        .iter()
        .find_map(|p| columns.iter().find(|c| c.eq_ignore_ascii_case(p)).copied())
        .or_else(|| {
            columns
                .iter()
                .find(|c| c.to_ascii_lowercase().ends_with("name"))
                .copied()
        })
}

/// SQL listing values of `reference` (with an optional label) that match
/// `search` in either, case-insensitively.
pub fn lookup_sql(
    engine: Engine,
    database: &str,
    reference: &ColumnRef,
    label: Option<&str>,
    search: &str,
) -> String {
    // MySQL's "schema" in a reference is the database.
    let table = match engine {
        Engine::MySql => quote::table(engine, &reference.schema, "", &reference.table),
        _ => quote::table(engine, database, &reference.schema, &reference.table),
    };
    let text = |column: &str| {
        let column = quote::ident(engine, column);
        match engine {
            Engine::Postgres => format!("CAST({column} AS TEXT)"),
            Engine::MySql => format!("CAST({column} AS CHAR)"),
            Engine::SqlServer => format!("CAST({column} AS NVARCHAR(4000))"),
        }
    };
    let key = quote::ident(engine, &reference.column);
    let select = match label {
        Some(label) => format!("{}, {}", text(&reference.column), text(label)),
        None => format!("{}, NULL", text(&reference.column)),
    };
    let mut filter = String::new();
    let search = search.trim();
    if !search.is_empty() {
        let pattern = quote::literal(engine, &format!("%{}%", search.to_lowercase()));
        let mut matches = vec![format!("LOWER({}) LIKE {pattern}", text(&reference.column))];
        if let Some(label) = label {
            matches.push(format!("LOWER({}) LIKE {pattern}", text(label)));
        }
        filter = format!(" WHERE {}", matches.join(" OR "));
    }
    match engine {
        Engine::SqlServer => {
            format!("SELECT TOP ({LOOKUP_LIMIT}) {select} FROM {table}{filter} ORDER BY {key}")
        }
        _ => format!("SELECT {select} FROM {table}{filter} ORDER BY {key} LIMIT {LOOKUP_LIMIT}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cv(column: &str, value: Option<&str>) -> ColumnValue {
        ColumnValue {
            column: column.into(),
            value: value.map(Into::into),
        }
    }

    fn target() -> TableTarget {
        TableTarget {
            database: "shop".into(),
            schema: "public".into(),
            name: "orders".into(),
        }
    }

    #[test]
    fn orders_deletes_updates_inserts_with_quoting() {
        let changes = [
            RowChange::Insert {
                values: vec![cv("id", Some("9")), cv("note", None)],
            },
            RowChange::Update {
                key: vec![cv("id", Some("71"))],
                set: vec![cv("note", Some("O'Brien")), cv("total", None)],
            },
            RowChange::Delete {
                key: vec![cv("id", Some("5")), cv("line", Some("2"))],
            },
        ];
        assert_eq!(
            statements(Engine::Postgres, &target(), &changes).unwrap(),
            [
                r#"DELETE FROM "public"."orders" WHERE "id" = '5' AND "line" = '2'"#,
                r#"UPDATE "public"."orders" SET "note" = 'O''Brien', "total" = NULL WHERE "id" = '71'"#,
                r#"INSERT INTO "public"."orders" ("id", "note") VALUES ('9', NULL)"#,
            ]
        );
        assert_eq!(
            statements(Engine::SqlServer, &target(), &changes[2..]).unwrap(),
            ["DELETE FROM [shop].[public].[orders] WHERE [id] = N'5' AND [line] = N'2'"]
        );
    }

    #[test]
    fn picks_a_label_column() {
        assert_eq!(label_column("id", ["id", "code", "Name"]), Some("Name"));
        assert_eq!(
            label_column("id", ["id", "country_name"]),
            Some("country_name")
        );
        assert_eq!(label_column("id", ["id", "rate"]), None);
    }

    #[test]
    fn builds_lookup_queries() {
        let reference = ColumnRef {
            schema: "public".into(),
            table: "customers".into(),
            column: "id".into(),
        };
        assert_eq!(
            lookup_sql(Engine::Postgres, "shop", &reference, Some("name"), " Ma "),
            r#"SELECT CAST("id" AS TEXT), CAST("name" AS TEXT) FROM "public"."customers" WHERE LOWER(CAST("id" AS TEXT)) LIKE '%ma%' OR LOWER(CAST("name" AS TEXT)) LIKE '%ma%' ORDER BY "id" LIMIT 50"#
        );
        let mysql = ColumnRef {
            schema: "shop".into(),
            ..reference.clone()
        };
        assert_eq!(
            lookup_sql(Engine::MySql, "other", &mysql, None, ""),
            "SELECT CAST(`id` AS CHAR), NULL FROM `shop`.`customers` ORDER BY `id` LIMIT 50"
        );
        assert_eq!(
            lookup_sql(Engine::SqlServer, "shop", &reference, None, ""),
            "SELECT TOP (50) CAST([id] AS NVARCHAR(4000)), NULL FROM [shop].[public].[customers] ORDER BY [id]"
        );
    }

    #[test]
    fn empty_inserts_use_defaults() {
        let insert = [RowChange::Insert { values: vec![] }];
        assert_eq!(
            statements(Engine::MySql, &target(), &insert).unwrap(),
            ["INSERT INTO `shop`.`orders` () VALUES ()"]
        );
        assert_eq!(
            statements(Engine::Postgres, &target(), &insert).unwrap(),
            [r#"INSERT INTO "public"."orders" DEFAULT VALUES"#]
        );
    }

    #[test]
    fn refuses_rows_without_a_usable_key() {
        for change in [
            RowChange::Delete { key: vec![] },
            RowChange::Delete {
                key: vec![cv("id", None)],
            },
            RowChange::Update {
                key: vec![cv("id", Some("1"))],
                set: vec![],
            },
        ] {
            assert_eq!(
                statements(Engine::Postgres, &target(), &[change])
                    .unwrap_err()
                    .kind,
                "invalid_edit"
            );
        }
    }
}

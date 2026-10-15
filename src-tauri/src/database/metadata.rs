//! Types for the database explorer. Metadata queries live with each engine.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Table,
    View,
    MaterializedView,
    ForeignTable,
}

impl ObjectKind {
    pub fn from_code(code: &str) -> Self {
        match code {
            "view" => Self::View,
            "materialized_view" => Self::MaterializedView,
            "foreign_table" => Self::ForeignTable,
            _ => Self::Table,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbObject {
    pub name: String,
    pub kind: ObjectKind,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default: Option<String>,
    /// Position in the primary key (1-based), if part of it.
    pub primary_key: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexInfo {
    pub name: String,
    /// Column names in key order; empty for expression-only indexes.
    pub columns: Vec<String>,
    pub unique: bool,
    pub primary: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStructure {
    pub columns: Vec<ColumnInfo>,
    /// `None` when the engine has no indexes for this kind of object
    /// (views), as opposed to an empty list.
    pub indexes: Option<Vec<IndexInfo>>,
}

/// Rows of text from a metadata query.
pub type TextRows = Vec<Vec<Option<String>>>;

pub(crate) fn column_rows(rows: TextRows) -> Vec<ColumnInfo> {
    rows.into_iter()
        .map(|mut row| {
            row.resize(5, None);
            let mut cells = row.into_iter();
            ColumnInfo {
                name: cells.next().flatten().unwrap_or_default(),
                data_type: cells.next().flatten().unwrap_or_default(),
                nullable: matches!(
                    cells.next().flatten().as_deref(),
                    Some("true" | "YES" | "1")
                ),
                default: cells.next().flatten(),
                primary_key: cells.next().flatten().and_then(|p| p.parse().ok()),
            }
        })
        .collect()
}

/// Rows of (name, unique, primary, "a, b, c").
pub(crate) fn index_rows(rows: TextRows) -> Vec<IndexInfo> {
    let truthy = |v: Option<String>| matches!(v.as_deref(), Some("true" | "1" | "t"));
    rows.into_iter()
        .map(|mut row| {
            row.resize(4, None);
            let mut cells = row.into_iter();
            let name = cells.next().flatten().unwrap_or_default();
            let unique = truthy(cells.next().flatten());
            let primary = truthy(cells.next().flatten());
            let columns = cells
                .next()
                .flatten()
                .map(|list| list.split(", ").map(str::to_owned).collect())
                .unwrap_or_default();
            IndexInfo {
                name,
                columns,
                unique,
                primary,
            }
        })
        .collect()
}

pub(crate) fn object_rows(rows: TextRows) -> Vec<DbObject> {
    rows.into_iter()
        .filter_map(|row| {
            let mut cells = row.into_iter();
            let name = cells.next().flatten()?;
            let kind = ObjectKind::from_code(cells.next().flatten().as_deref().unwrap_or("table"));
            Some(DbObject { name, kind })
        })
        .collect()
}

pub(crate) fn names(rows: TextRows) -> Vec<String> {
    rows.into_iter()
        .filter_map(|row| row.into_iter().next().flatten())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_column_rows_from_any_engine() {
        let rows = vec![
            vec![
                Some("id".into()),
                Some("bigint".into()),
                Some("NO".into()),
                None,
                Some("1".into()),
            ],
            vec![
                Some("note".into()),
                Some("text".into()),
                Some("true".into()),
                None,
                None,
            ],
        ];
        let columns = column_rows(rows);
        assert!(!columns[0].nullable);
        assert_eq!(columns[0].primary_key, Some(1));
        assert!(columns[1].nullable);
        assert_eq!(columns[1].primary_key, None);
    }

    #[test]
    fn parses_index_rows() {
        let rows = vec![vec![
            Some("orders_pkey".into()),
            Some("t".into()),
            Some("true".into()),
            Some("id, region".into()),
        ]];
        let index = &index_rows(rows)[0];
        assert!(index.unique && index.primary);
        assert_eq!(index.columns, vec!["id", "region"]);
    }
}

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
    /// Allowed values of an enum column (PostgreSQL enum types, MySQL ENUM).
    pub enum_values: Option<Vec<String>>,
    /// The column a single-column foreign key points to.
    pub references: Option<ColumnRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnRef {
    /// Schema (PostgreSQL, SQL Server) or database (MySQL).
    pub schema: String,
    pub table: String,
    pub column: String,
}

/// Separates list items in metadata query results (ASCII unit separator,
/// which can't appear in names or enum labels in practice).
pub(crate) const SEP: char = '\u{1f}';

fn references(text: Option<String>) -> Option<ColumnRef> {
    let text = text?;
    let mut parts = text.split(SEP).map(str::to_owned);
    Some(ColumnRef {
        schema: parts.next()?,
        table: parts.next()?,
        column: parts.next()?,
    })
}

/// Values of a MySQL `enum('a','b''c')` column type.
fn mysql_enum(column_type: &str) -> Option<Vec<String>> {
    let body = column_type
        .strip_prefix("enum(")
        .or_else(|| column_type.strip_prefix("ENUM("))?
        .strip_suffix(')')?;
    let mut values = Vec::new();
    let mut chars = body.chars().peekable();
    while chars.peek().is_some() {
        if chars.next()? != '\'' {
            return None;
        }
        let mut value = String::new();
        loop {
            match chars.next()? {
                '\'' if chars.peek() == Some(&'\'') => {
                    chars.next();
                    value.push('\'');
                }
                '\'' => break,
                '\\' => value.push(chars.next()?),
                c => value.push(c),
            }
        }
        values.push(value);
        match chars.next() {
            Some(',') | None => {}
            _ => return None,
        }
    }
    Some(values)
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
            row.resize(7, None);
            let mut cells = row.into_iter();
            let name = cells.next().flatten().unwrap_or_default();
            let data_type = cells.next().flatten().unwrap_or_default();
            let nullable = matches!(
                cells.next().flatten().as_deref(),
                Some("true" | "YES" | "1")
            );
            let default = cells.next().flatten();
            let primary_key = cells.next().flatten().and_then(|p| p.parse().ok());
            let enum_values = cells
                .next()
                .flatten()
                .map(|list| list.split(SEP).map(str::to_owned).collect())
                .or_else(|| mysql_enum(&data_type));
            ColumnInfo {
                name,
                data_type,
                nullable,
                default,
                primary_key,
                enum_values,
                references: references(cells.next().flatten()),
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
    fn reads_enum_values_and_foreign_keys() {
        let sep = SEP.to_string();
        let columns = column_rows(vec![
            vec![
                Some("status".into()),
                Some("order_status".into()),
                Some("NO".into()),
                None,
                None,
                Some(["new", "paid"].join(&sep)),
                None,
            ],
            vec![
                Some("kind".into()),
                Some(r"enum('a','it''s','b\\c')".into()),
                Some("YES".into()),
                None,
                None,
                None,
                Some(["shop", "customers", "id"].join(&sep)),
            ],
        ]);
        assert_eq!(
            columns[0].enum_values.as_deref(),
            Some(&["new".to_owned(), "paid".to_owned()][..])
        );
        assert_eq!(columns[0].references, None);
        assert_eq!(
            columns[1].enum_values,
            Some(vec!["a".into(), "it's".into(), r"b\c".into()])
        );
        assert_eq!(
            columns[1].references,
            Some(ColumnRef {
                schema: "shop".into(),
                table: "customers".into(),
                column: "id".into(),
            })
        );
        assert_eq!(mysql_enum("varchar(20)"), None);
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

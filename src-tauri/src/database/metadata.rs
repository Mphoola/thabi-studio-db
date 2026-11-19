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
    /// MySQL's own `CREATE TABLE`, when the engine will say it.
    ///
    /// Nothing else has an equivalent, and for a dump this is the difference
    /// between the server's words and our reconstruction of them.
    #[serde(default)]
    pub show_create: Option<String>,
    /// Foreign keys (any number of columns) and check constraints, by name,
    /// so the designer can drop them. `None` where the object has none to
    /// have (views).
    #[serde(default)]
    pub constraints: Option<Vec<ConstraintInfo>>,
}

/// A foreign key or check constraint on a table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstraintInfo {
    pub name: String,
    /// "foreignKey" or "check".
    pub kind: String,
    /// The constrained columns, in key order (foreign keys).
    pub columns: Vec<String>,
    /// Where a foreign key points: schema (PostgreSQL, SQL Server) or
    /// database (MySQL), table and columns in the same order.
    pub referenced_schema: String,
    pub referenced_table: String,
    pub referenced_columns: Vec<String>,
    pub on_delete: String,
    pub on_update: String,
    /// A check's condition, as the server prints it.
    pub definition: String,
}

/// The query for a table's foreign keys and checks. Rows are (name, kind,
/// columns, referenced schema, table, columns, on delete, on update,
/// definition), lists separated by [`SEP`]. Bound values: schema (or MySQL
/// database), then table; MySQL takes them twice.
pub(crate) fn constraints_sql(engine: super::Engine, database: &str) -> String {
    match engine {
        super::Engine::Postgres => "SELECT con.conname::text,
                    CASE con.contype WHEN 'f' THEN 'foreignKey' ELSE 'check' END,
                    (SELECT string_agg(a.attname, chr(31) ORDER BY k.ord)
                     FROM unnest(con.conkey) WITH ORDINALITY AS k(attnum, ord)
                     JOIN pg_attribute a ON a.attrelid = con.conrelid AND a.attnum = k.attnum),
                    rn.nspname::text, rc.relname::text,
                    (SELECT string_agg(a.attname, chr(31) ORDER BY k.ord)
                     FROM unnest(con.confkey) WITH ORDINALITY AS k(attnum, ord)
                     JOIN pg_attribute a ON a.attrelid = con.confrelid AND a.attnum = k.attnum),
                    CASE con.confdeltype WHEN 'a' THEN 'NO ACTION' WHEN 'r' THEN 'RESTRICT'
                         WHEN 'c' THEN 'CASCADE' WHEN 'n' THEN 'SET NULL' WHEN 'd' THEN 'SET DEFAULT' END,
                    CASE con.confupdtype WHEN 'a' THEN 'NO ACTION' WHEN 'r' THEN 'RESTRICT'
                         WHEN 'c' THEN 'CASCADE' WHEN 'n' THEN 'SET NULL' WHEN 'd' THEN 'SET DEFAULT' END,
                    CASE WHEN con.contype = 'c' THEN pg_get_constraintdef(con.oid) END
             FROM pg_constraint con
             JOIN pg_class c ON c.oid = con.conrelid
             JOIN pg_namespace n ON n.oid = c.relnamespace
             LEFT JOIN pg_class rc ON rc.oid = con.confrelid
             LEFT JOIN pg_namespace rn ON rn.oid = rc.relnamespace
             WHERE n.nspname = $1 AND c.relname = $2 AND con.contype IN ('f', 'c')
             ORDER BY con.contype DESC, con.conname"
            .to_owned(),
        super::Engine::MySql => "SELECT CONVERT(rc.CONSTRAINT_NAME USING utf8mb4), 'foreignKey',
                    CONVERT(GROUP_CONCAT(k.COLUMN_NAME ORDER BY k.ORDINAL_POSITION SEPARATOR '\x1f') USING utf8mb4),
                    CONVERT(MAX(k.REFERENCED_TABLE_SCHEMA) USING utf8mb4),
                    CONVERT(MAX(k.REFERENCED_TABLE_NAME) USING utf8mb4),
                    CONVERT(GROUP_CONCAT(k.REFERENCED_COLUMN_NAME ORDER BY k.ORDINAL_POSITION SEPARATOR '\x1f') USING utf8mb4),
                    CONVERT(rc.DELETE_RULE USING utf8mb4), CONVERT(rc.UPDATE_RULE USING utf8mb4), NULL
             FROM information_schema.REFERENTIAL_CONSTRAINTS rc
             JOIN information_schema.KEY_COLUMN_USAGE k
               ON k.CONSTRAINT_SCHEMA = rc.CONSTRAINT_SCHEMA AND k.CONSTRAINT_NAME = rc.CONSTRAINT_NAME
              AND k.TABLE_NAME = rc.TABLE_NAME
             WHERE rc.CONSTRAINT_SCHEMA = ? AND rc.TABLE_NAME = ?
             GROUP BY rc.CONSTRAINT_NAME, rc.DELETE_RULE, rc.UPDATE_RULE"
            .to_owned(),
        super::Engine::SqlServer => {
            let db = super::quote::ident(super::Engine::SqlServer, database);
            format!(
                "SELECT fk.name, 'foreignKey',
                        STRING_AGG(pc.name, CHAR(31)) WITHIN GROUP (ORDER BY fkc.constraint_column_id),
                        MAX(rs.name), MAX(ro.name),
                        STRING_AGG(rcol.name, CHAR(31)) WITHIN GROUP (ORDER BY fkc.constraint_column_id),
                        REPLACE(MAX(fk.delete_referential_action_desc), '_', ' '),
                        REPLACE(MAX(fk.update_referential_action_desc), '_', ' '),
                        NULL
                 FROM {db}.sys.foreign_keys fk
                 JOIN {db}.sys.objects o ON o.object_id = fk.parent_object_id
                 JOIN {db}.sys.schemas s ON s.schema_id = o.schema_id
                 JOIN {db}.sys.foreign_key_columns fkc ON fkc.constraint_object_id = fk.object_id
                 JOIN {db}.sys.columns pc ON pc.object_id = fkc.parent_object_id AND pc.column_id = fkc.parent_column_id
                 JOIN {db}.sys.objects ro ON ro.object_id = fk.referenced_object_id
                 JOIN {db}.sys.schemas rs ON rs.schema_id = ro.schema_id
                 JOIN {db}.sys.columns rcol ON rcol.object_id = fkc.referenced_object_id AND rcol.column_id = fkc.referenced_column_id
                 WHERE s.name = @P1 AND o.name = @P2
                 GROUP BY fk.name
                 UNION ALL
                 SELECT cc.name, 'check', NULL, NULL, NULL, NULL, NULL, NULL, cc.definition
                 FROM {db}.sys.check_constraints cc
                 JOIN {db}.sys.objects o ON o.object_id = cc.parent_object_id
                 JOIN {db}.sys.schemas s ON s.schema_id = o.schema_id
                 WHERE s.name = @P1 AND o.name = @P2"
            )
        }
    }
}

/// MySQL's check constraints, apart from [`constraints_sql`]:
/// `information_schema.CHECK_CONSTRAINTS` only exists from MySQL 8.0.16, so
/// on older servers (MySQL 5.7, as MAMP ships) this fails and is skipped.
pub(crate) fn mysql_checks_sql() -> &'static str {
    "SELECT CONVERT(cc.CONSTRAINT_NAME USING utf8mb4), 'check', NULL, NULL, NULL, NULL, NULL, NULL,
                    CONVERT(cc.CHECK_CLAUSE USING utf8mb4)
             FROM information_schema.CHECK_CONSTRAINTS cc
             JOIN information_schema.TABLE_CONSTRAINTS tc
               ON tc.CONSTRAINT_SCHEMA = cc.CONSTRAINT_SCHEMA AND tc.CONSTRAINT_NAME = cc.CONSTRAINT_NAME
             WHERE tc.TABLE_SCHEMA = ? AND tc.TABLE_NAME = ? AND tc.CONSTRAINT_TYPE = 'CHECK'"
}

/// Rows from [`constraints_sql`].
pub(crate) fn constraint_rows(rows: TextRows) -> Vec<ConstraintInfo> {
    let list = |v: Option<String>| -> Vec<String> {
        v.map(|text| text.split(SEP).map(str::to_owned).collect())
            .unwrap_or_default()
    };
    let mut out: Vec<ConstraintInfo> = rows
        .into_iter()
        .map(|mut row| {
            row.resize(9, None);
            let mut cells = row.into_iter();
            let mut next = || cells.next().flatten();
            ConstraintInfo {
                name: next().unwrap_or_default(),
                kind: next().unwrap_or_default(),
                columns: list(next()),
                referenced_schema: next().unwrap_or_default(),
                referenced_table: next().unwrap_or_default(),
                referenced_columns: list(next()),
                on_delete: next().unwrap_or_default(),
                on_update: next().unwrap_or_default(),
                definition: next().unwrap_or_default(),
            }
        })
        .collect();
    out.sort_by(|a, b| b.kind.cmp(&a.kind).then(a.name.cmp(&b.name)));
    out
}

/// A single-column foreign key in another table that points at this one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Referencing {
    /// Schema (PostgreSQL, SQL Server) or database (MySQL) of the child table.
    pub schema: String,
    pub table: String,
    /// The child's foreign key column.
    pub column: String,
    /// The column in this table it points at.
    pub referenced_column: String,
}

/// SQL listing single-column foreign keys that point at a table. Binds the
/// schema (or MySQL database) and table name as $1/?/@P1 and $2/?/@P2.
pub(crate) fn referencing_sql(engine: super::Engine, database: &str) -> String {
    use super::{Engine, quote};
    match engine {
        Engine::Postgres => "SELECT cn.nspname::text, cc.relname::text, ca.attname::text, pa.attname::text
             FROM pg_constraint con
             JOIN pg_class pc ON pc.oid = con.confrelid
             JOIN pg_namespace pn ON pn.oid = pc.relnamespace
             JOIN pg_class cc ON cc.oid = con.conrelid
             JOIN pg_namespace cn ON cn.oid = cc.relnamespace
             JOIN pg_attribute ca ON ca.attrelid = con.conrelid AND ca.attnum = con.conkey[1]
             JOIN pg_attribute pa ON pa.attrelid = con.confrelid AND pa.attnum = con.confkey[1]
             WHERE con.contype = 'f' AND cardinality(con.conkey) = 1
               AND pn.nspname = $1 AND pc.relname = $2
             ORDER BY 1, 2, 3"
            .to_owned(),
        Engine::MySql => "SELECT CONVERT(k.TABLE_SCHEMA USING utf8mb4), CONVERT(k.TABLE_NAME USING utf8mb4),
                    CONVERT(k.COLUMN_NAME USING utf8mb4), CONVERT(k.REFERENCED_COLUMN_NAME USING utf8mb4)
             FROM information_schema.KEY_COLUMN_USAGE k
             WHERE k.REFERENCED_TABLE_SCHEMA = ? AND k.REFERENCED_TABLE_NAME = ?
               AND (SELECT COUNT(*) FROM information_schema.KEY_COLUMN_USAGE k2
                    WHERE k2.CONSTRAINT_SCHEMA = k.CONSTRAINT_SCHEMA
                      AND k2.TABLE_NAME = k.TABLE_NAME
                      AND k2.CONSTRAINT_NAME = k.CONSTRAINT_NAME) = 1
             ORDER BY 1, 2, 3"
            .to_owned(),
        Engine::SqlServer => {
            let db = quote::ident(engine, database);
            format!(
                "SELECT cs.name, co.name, cc.name, pc.name
                 FROM {db}.sys.foreign_key_columns f
                 JOIN {db}.sys.objects po ON po.object_id = f.referenced_object_id
                 JOIN {db}.sys.schemas ps ON ps.schema_id = po.schema_id
                 JOIN {db}.sys.objects co ON co.object_id = f.parent_object_id
                 JOIN {db}.sys.schemas cs ON cs.schema_id = co.schema_id
                 JOIN {db}.sys.columns cc ON cc.object_id = f.parent_object_id AND cc.column_id = f.parent_column_id
                 JOIN {db}.sys.columns pc ON pc.object_id = f.referenced_object_id AND pc.column_id = f.referenced_column_id
                 WHERE ps.name = @P1 AND po.name = @P2
                   AND (SELECT COUNT(*) FROM {db}.sys.foreign_key_columns x
                        WHERE x.constraint_object_id = f.constraint_object_id) = 1
                 ORDER BY 1, 2, 3"
            )
        }
    }
}

pub(crate) fn referencing_rows(rows: TextRows) -> Vec<Referencing> {
    rows.into_iter()
        .filter_map(|row| {
            let mut cells = row.into_iter();
            Some(Referencing {
                schema: cells.next()??,
                table: cells.next()??,
                column: cells.next()??,
                referenced_column: cells.next()??,
            })
        })
        .collect()
}

/// A table and its columns, for editor autocomplete.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionTable {
    /// Schema (PostgreSQL, SQL Server) or "" (MySQL, where the database is it).
    pub schema: String,
    pub table: String,
    pub columns: Vec<String>,
}

/// Upper bound on columns read for autocomplete, so a huge catalog can't
/// stall the editor; the rest simply isn't suggested.
pub(crate) const COMPLETION_COLUMNS: usize = 50_000;

/// SQL listing every column of every table and view in a database. MySQL
/// binds the database as ?.
pub(crate) fn completion_sql(engine: super::Engine, database: &str) -> String {
    use super::{Engine, quote};
    match engine {
        Engine::Postgres => format!(
            "SELECT table_schema::text, table_name::text, column_name::text
             FROM information_schema.columns
             WHERE table_schema NOT IN ('pg_catalog', 'information_schema')
               AND table_schema NOT LIKE 'pg_toast%'
             ORDER BY table_schema, table_name, ordinal_position
             LIMIT {COMPLETION_COLUMNS}"
        ),
        Engine::MySql => format!(
            "SELECT '', CONVERT(TABLE_NAME USING utf8mb4), CONVERT(COLUMN_NAME USING utf8mb4)
             FROM information_schema.COLUMNS WHERE TABLE_SCHEMA = ?
             ORDER BY TABLE_NAME, ORDINAL_POSITION
             LIMIT {COMPLETION_COLUMNS}"
        ),
        Engine::SqlServer => {
            let db = quote::ident(engine, database);
            format!(
                "SELECT TOP ({COMPLETION_COLUMNS}) TABLE_SCHEMA, TABLE_NAME, COLUMN_NAME
                 FROM {db}.INFORMATION_SCHEMA.COLUMNS
                 ORDER BY TABLE_SCHEMA, TABLE_NAME, ORDINAL_POSITION"
            )
        }
    }
}

/// Rows of (schema, table, column), grouped into tables in order.
pub(crate) fn completion_rows(rows: TextRows) -> Vec<CompletionTable> {
    let mut tables: Vec<CompletionTable> = Vec::new();
    for row in rows {
        let mut cells = row.into_iter();
        let (Some(schema), Some(Some(table)), Some(Some(column))) =
            (cells.next(), cells.next(), cells.next())
        else {
            continue;
        };
        let schema = schema.unwrap_or_default();
        match tables.last_mut() {
            Some(last) if last.schema == schema && last.table == table => last.columns.push(column),
            _ => tables.push(CompletionTable {
                schema,
                table,
                columns: vec![column],
            }),
        }
    }
    tables
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

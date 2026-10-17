//! What the ER diagram draws, and the queries that read it.
//!
//! Three small queries rather than one joined one: a join across a foreign
//! key repeats every column row once per key column, which on a schema with
//! wide tables is thousands of rows to throw away. Separate queries stay
//! legible and each one is only as complicated as its engine requires.
//!
//!
//! Constraint names are read as well as columns, because two foreign keys
//! between the same pair of tables are two edges, not one, and a diagram that
//! merges them is wrong in a way nobody notices.

use super::Engine;
use super::metadata::TextRows;

/// One column in a table box.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
}

/// One table box.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramTable {
    pub name: String,
    pub columns: Vec<DiagramColumn>,
}

/// One line between two tables.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramForeignKey {
    pub name: String,
    pub from_table: String,
    /// As many names as the key has columns, in key order.
    pub from_columns: Vec<String>,
    pub to_table: String,
    pub to_columns: Vec<String>,
}

/// Everything one diagram draws.
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagram {
    pub tables: Vec<DiagramTable>,
    pub foreign_keys: Vec<DiagramForeignKey>,
}

/// Ceiling on tables drawn. A schema larger than this is past the point where
/// a picture helps, and the limit is said out loud by the caller rather than
/// quietly truncating.
pub const DIAGRAM_TABLES: usize = 500;

/// Columns of every table in a schema: (table, column, type, nullable).
pub(crate) fn columns_sql(engine: Engine) -> String {
    match engine {
        // TABLE_TYPE lives in TABLES, not COLUMNS — the server says so.
        Engine::MySql => "SELECT CONVERT(c.TABLE_NAME USING utf8mb4),
                                CONVERT(c.COLUMN_NAME USING utf8mb4),
                                CONVERT(c.COLUMN_TYPE USING utf8mb4),
                                c.IS_NULLABLE
                         FROM information_schema.COLUMNS c
                         JOIN information_schema.TABLES t
                           ON t.TABLE_SCHEMA = c.TABLE_SCHEMA
                          AND t.TABLE_NAME = c.TABLE_NAME
                         WHERE c.TABLE_SCHEMA = ?
                           AND t.TABLE_TYPE = 'BASE TABLE'
                         ORDER BY c.TABLE_NAME, c.ORDINAL_POSITION"
            .to_owned(),
        Engine::Postgres => "SELECT c.table_name::text, c.column_name::text,
                                    c.data_type::text, c.is_nullable::text
                             FROM information_schema.columns c
                             JOIN information_schema.tables t
                               ON t.table_schema = c.table_schema
                              AND t.table_name = c.table_name
                             WHERE c.table_schema = $1
                               AND t.table_type = 'BASE TABLE'
                             ORDER BY c.table_name, c.ordinal_position"
            .to_owned(),
        Engine::SqlServer => "SELECT t.name, c.name, ty.name + coalesce(
                        '(' + case
                            when c.max_length = -1 then 'max'
                            when ty.name IN ('nchar','nvarchar','ntext') then
                                 cast(c.max_length / 2 as varchar(10))
                            else cast(c.max_length as varchar(10)) end + ')',
                        '') AS full_type,
                     cast(c.is_nullable as varchar(1))
             FROM sys.tables t
             JOIN sys.columns c ON c.object_id = t.object_id
             JOIN sys.types ty ON ty.user_type_id = c.user_type_id
             JOIN sys.schemas s ON s.schema_id = t.schema_id
             WHERE s.name = @p1
             ORDER BY t.name, c.column_id"
            .to_owned(),
    }
}

/// Primary key columns: (table, column).
pub(crate) fn primary_keys_sql(engine: Engine) -> String {
    match engine {
        Engine::MySql => "SELECT CONVERT(TABLE_NAME USING utf8mb4),
                                CONVERT(COLUMN_NAME USING utf8mb4)
                         FROM information_schema.KEY_COLUMN_USAGE
                         WHERE TABLE_SCHEMA = ? AND CONSTRAINT_NAME = 'PRIMARY'
                         ORDER BY TABLE_NAME, ORDINAL_POSITION"
            .to_owned(),
        Engine::Postgres => "SELECT tc.relname::text, a.attname::text
                             FROM pg_constraint con
                             JOIN pg_class tc ON tc.oid = con.conrelid
                             JOIN pg_namespace n ON n.oid = tc.relnamespace
                             JOIN LATERAL unnest(con.conkey) WITH ORDINALITY
                                  AS k(attnum, ord) ON TRUE
                             JOIN pg_attribute a
                               ON a.attrelid = con.conrelid AND a.attnum = k.attnum
                             WHERE con.contype = 'p' AND n.nspname = $1
                             ORDER BY tc.relname, con.conname, k.ord"
            .to_owned(),
        Engine::SqlServer => "SELECT t.name, c.name
             FROM sys.key_constraints kc
             JOIN sys.tables t ON t.object_id = kc.parent_object_id
             JOIN sys.index_columns ic
               ON ic.object_id = kc.parent_object_id AND ic.index_id = kc.unique_index_id
             JOIN sys.columns c
               ON c.object_id = ic.object_id AND c.column_id = ic.column_id
             JOIN sys.schemas s ON s.schema_id = t.schema_id
             WHERE kc.type = 'PK' AND s.name = @p1
             ORDER BY t.name, kc.name, ic.key_ordinal"
            .to_owned(),
    }
}

/// Foreign keys: (constraint, from table, from column, to table, to column).
///
/// Ordered by constraint then by position within it, because a composite key's
/// columns are only paired correctly if they arrive in key order.
pub(crate) fn foreign_keys_sql(engine: Engine) -> String {
    match engine {
        Engine::MySql => "SELECT CONVERT(k.CONSTRAINT_NAME USING utf8mb4),
                                CONVERT(k.TABLE_NAME USING utf8mb4),
                                CONVERT(k.COLUMN_NAME USING utf8mb4),
                                CONVERT(k.REFERENCED_TABLE_NAME USING utf8mb4),
                                CONVERT(k.REFERENCED_COLUMN_NAME USING utf8mb4)
                         FROM information_schema.KEY_COLUMN_USAGE k
                         WHERE k.TABLE_SCHEMA = ?
                           AND k.REFERENCED_TABLE_NAME IS NOT NULL
                         ORDER BY k.TABLE_NAME, k.CONSTRAINT_NAME, k.ORDINAL_POSITION"
            .to_owned(),
        // pg_constraint rather than information_schema, because only
        // constraint_column_usage is available there and it does not preserve
        // the order of a composite key's columns.
        Engine::Postgres => "SELECT con.conname::text,
                                    tc.relname::text,
                                    a.attname::text,
                                    rc.relname::text,
                                    ra.attname::text
                             FROM pg_constraint con
                             JOIN pg_class tc ON tc.oid = con.conrelid
                             JOIN pg_class rc ON rc.oid = con.confrelid
                             JOIN pg_namespace n ON n.oid = tc.relnamespace
                             JOIN LATERAL unnest(con.conkey, con.confkey)
                                  WITH ORDINALITY AS k(attnum, fattnum, ord) ON TRUE
                             JOIN pg_attribute a
                               ON a.attrelid = con.conrelid AND a.attnum = k.attnum
                             JOIN pg_attribute ra
                               ON ra.attrelid = con.confrelid AND ra.attnum = k.fattnum
                             WHERE con.contype = 'f' AND n.nspname = $1
                             ORDER BY tc.relname, con.conname, k.ord"
            .to_owned(),
        Engine::SqlServer => "SELECT fk.name, tp.name, cp.name, tr.name, cr.name
             FROM sys.foreign_keys fk
             JOIN sys.tables tp ON tp.object_id = fk.parent_object_id
             JOIN sys.tables tr ON tr.object_id = fk.referenced_object_id
             JOIN sys.foreign_key_columns fkc
               ON fkc.constraint_object_id = fk.object_id
             JOIN sys.columns cp
               ON cp.object_id = fkc.parent_object_id
              AND cp.column_id = fkc.parent_column_id
             JOIN sys.columns cr
               ON cr.object_id = fkc.referenced_object_id
              AND cr.column_id = fkc.referenced_column_id
             JOIN sys.schemas s ON s.schema_id = tp.schema_id
             WHERE s.name = @p1
             ORDER BY tp.name, fk.name, fkc.constraint_column_id"
            .to_owned(),
    }
}

/// How many binds the column and key queries take: the owning schema, or the
/// database itself on MySQL where the database is the schema.
pub(crate) fn owner_of<'a>(engine: Engine, database: &'a str, schema: &'a str) -> &'a str {
    if engine == Engine::MySql {
        database
    } else {
        schema
    }
}

/// Turns the three result sets into the one thing the diagram draws.
pub(crate) fn build(columns: TextRows, primary_keys: TextRows, foreign_keys: TextRows) -> Diagram {
    let keyed: Vec<(String, String)> = primary_keys
        .into_iter()
        .filter_map(|mut row| {
            row.resize(2, None);
            let mut cells = row.into_iter();
            match (cells.next(), cells.next()) {
                (Some(Some(table)), Some(Some(column))) => Some((table, column)),
                _ => None,
            }
        })
        .collect();

    let mut tables: Vec<DiagramTable> = Vec::new();
    for mut row in columns {
        row.resize(4, None);
        let mut cells = row.into_iter();
        let (Some(table), Some(column), Some(data_type), Some(nullable)) =
            (cells.next(), cells.next(), cells.next(), cells.next())
        else {
            continue;
        };
        let (Some(table), Some(column)) = (table, column) else {
            continue;
        };
        let data_type = data_type.unwrap_or_default();
        let primary_key = keyed
            .iter()
            .any(|(one_table, one_column)| one_table == &table && one_column == &column);

        // The query orders by table, so a row either continues the last table
        // or starts a new one.
        match tables.last_mut() {
            Some(last) if last.name == table => last.columns.push(DiagramColumn {
                name: column,
                data_type,
                nullable: nullable.as_deref() != Some("NO"),
                primary_key,
            }),
            _ => tables.push(DiagramTable {
                name: table,
                columns: vec![DiagramColumn {
                    name: column,
                    data_type,
                    nullable: nullable.as_deref() != Some("NO"),
                    primary_key,
                }],
            }),
        }
    }

    let mut edges: Vec<DiagramForeignKey> = Vec::new();
    for mut row in foreign_keys {
        row.resize(5, None);
        let mut cells = row.into_iter();
        let (Some(name), Some(from_table), Some(from_column), Some(to_table), Some(to_column)) = (
            cells.next(),
            cells.next(),
            cells.next(),
            cells.next(),
            cells.next(),
        ) else {
            continue;
        };
        let (Some(name), Some(from_table), Some(from_column), Some(to_table), Some(to_column)) =
            (name, from_table, from_column, to_table, to_column)
        else {
            continue;
        };
        match edges.last_mut() {
            Some(last) if last.name == name => {
                last.from_columns.push(from_column);
                last.to_columns.push(to_column);
            }
            _ => edges.push(DiagramForeignKey {
                name,
                from_table,
                from_columns: vec![from_column],
                to_table,
                to_columns: vec![to_column],
            }),
        }
    }

    Diagram {
        tables,
        foreign_keys: edges,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::quote;

    fn cell(value: &str) -> Vec<Option<String>> {
        vec![Some(value.to_owned())]
    }

    #[test]
    fn groups_columns_into_tables_in_order() {
        let rows = vec![
            {
                let mut row = cell("users");
                row.extend(cell("id"));
                row.extend(cell("bigint"));
                row.extend(cell("NO"));
                row
            },
            {
                let mut row = cell("users");
                row.extend(cell("email"));
                row.extend(cell("text"));
                row.extend(cell("YES"));
                row
            },
            {
                let mut row = cell("orders");
                row.extend(cell("id"));
                row.extend(cell("bigint"));
                row.extend(cell("NO"));
                row
            },
        ];
        let diagram = build(rows, Vec::new(), Vec::new());
        assert_eq!(diagram.tables.len(), 2);
        assert_eq!(diagram.tables[0].name, "users");
        assert_eq!(diagram.tables[0].columns.len(), 2);
        assert_eq!(diagram.tables[0].columns[1].name, "email");
        // id is NOT NULL, email is. Asserting the wrong one of the two would
        // have passed whether or not the flag was read at all.
        assert!(!diagram.tables[0].columns[0].nullable);
        assert!(diagram.tables[0].columns[1].nullable);
        assert_eq!(diagram.tables[1].name, "orders");
        assert_eq!(diagram.tables[1].columns.len(), 1);
    }

    #[test]
    fn marks_primary_key_columns() {
        let rows = vec![
            {
                let mut row = cell("users");
                row.extend(cell("id"));
                row.extend(cell("bigint"));
                row.extend(cell("NO"));
                row
            },
            {
                let mut row = cell("users");
                row.extend(cell("email"));
                row.extend(cell("text"));
                row.extend(cell("YES"));
                row
            },
        ];
        let keys = vec![{
            let mut row = cell("users");
            row.extend(cell("id"));
            row
        }];
        let diagram = build(rows, keys, Vec::new());
        assert!(diagram.tables[0].columns[0].primary_key);
        assert!(!diagram.tables[0].columns[1].primary_key);
    }

    /// Two keys between the same two tables are two lines. Merging them draws
    /// a relationship that does not exist.
    #[test]
    fn keeps_two_keys_between_the_same_tables_apart() {
        let rows: TextRows = [
            ["orders_id_fk", "orders", "customer_id", "customers", "id"],
            [
                "region_id_fk",
                "orders",
                "region_id",
                "customers",
                "region_id",
            ],
        ]
        .into_iter()
        .map(|cells| -> Vec<Option<String>> {
            cells.into_iter().map(|one| Some(one.to_owned())).collect()
        })
        .collect();
        let diagram = build(Vec::new(), Vec::new(), rows);
        assert_eq!(diagram.foreign_keys.len(), 2);
        assert_eq!(diagram.foreign_keys[0].name, "orders_id_fk");
        assert_eq!(diagram.foreign_keys[1].to_columns, ["region_id".to_owned()]);
    }

    /// The columns must arrive in key order or the pairing is nonsense:
    /// (a, b) -> (x, y) is not (a, b) -> (y, x).
    #[test]
    fn pairs_composite_key_columns_in_order() {
        let rows: TextRows = [
            ["fk", "orders", "tenant_id", "tenants", "tenant_id"],
            ["fk", "orders", "site_id", "tenants", "site_id"],
        ]
        .into_iter()
        .map(|cells| -> Vec<Option<String>> {
            cells.into_iter().map(|one| Some(one.to_owned())).collect()
        })
        .collect();
        let diagram = build(Vec::new(), Vec::new(), rows);
        assert_eq!(diagram.foreign_keys.len(), 1);
        assert_eq!(
            diagram.foreign_keys[0].from_columns,
            ["tenant_id".to_owned(), "site_id".to_owned()]
        );
        assert_eq!(
            diagram.foreign_keys[0].to_columns,
            ["tenant_id".to_owned(), "site_id".to_owned()]
        );
    }

    /// A key naming a column that isn't drawn is a broken edge, not a crash.
    #[test]
    fn keeps_a_key_pointing_at_a_table_it_never_read() {
        let rows: TextRows = [
            ["fk", "orders", "customer_id", "customers", "id"],
            ["fk2", "orders", "ghost_id", "nowhere", "id"],
        ]
        .into_iter()
        .map(|cells| -> Vec<Option<String>> {
            cells.into_iter().map(|one| Some(one.to_owned())).collect()
        })
        .collect();
        let diagram = build(Vec::new(), Vec::new(), rows);
        assert_eq!(diagram.foreign_keys.len(), 2);
        assert!(diagram.tables.is_empty());
    }

    #[test]
    fn skips_short_rows_rather_than_guessing() {
        let rows = vec![vec![Some("users".to_owned())], Vec::new()];
        let diagram = build(rows, vec![vec![Some("users".to_owned())]], Vec::new());
        assert!(diagram.tables.is_empty());
        assert!(diagram.foreign_keys.is_empty());
    }

    #[test]
    fn mysql_takes_the_database_where_the_others_take_the_schema() {
        assert_eq!(owner_of(Engine::MySql, "shop", "ignored"), "shop");
        assert_eq!(owner_of(Engine::Postgres, "shop", "public"), "public");
        assert_eq!(owner_of(Engine::SqlServer, "shop", "dbo"), "dbo");
    }

    #[test]
    fn every_engine_binds_the_owner_as_the_first_parameter() {
        for engine in [Engine::MySql, Engine::Postgres, Engine::SqlServer] {
            let sql = columns_sql(engine);
            let marker = match engine {
                Engine::MySql => "?",
                Engine::Postgres => "$1",
                Engine::SqlServer => "@p1",
            };
            assert!(
                sql.contains(marker),
                "{engine:?} does not bind its owner as {marker}"
            );
        }
    }

    #[test]
    fn sql_server_types_keep_their_length() {
        // max_length counts bytes, so nvarchar(20) is 40 and must be halved,
        // or the diagram tells people their column is nvarchar(40).
        assert!(columns_sql(Engine::SqlServer).contains("max_length / 2"));
        assert!(quote::ident(Engine::SqlServer, "x").starts_with('['));
    }
}

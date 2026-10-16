//! `CREATE TABLE`, and the actions that remove or rename one, per engine.

use super::super::{Engine, quote};
/// A column as the designer describes it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct ColumnSpec {
    pub name: String,
    /// The engine's own type name, as the structure view reported it.
    pub data_type: String,
    pub nullable: bool,
    /// A default expression as written, or empty for none. Kept as text
    /// because `now()` and `'a'` are not the same kind of thing as `0`.
    pub default: String,
    /// A comment, where the engine has them.
    pub comment: String,
    /// Primary key, which is written inside CREATE TABLE on all three engines.
    pub primary_key: bool,
    pub unique: bool,
}

impl ColumnSpec {
    pub(super) fn column_line(&self, engine: Engine) -> String {
        let mut line = format!(
            "{} {}",
            quote::ident(engine, &self.name),
            self.data_type.trim()
        );
        if !self.nullable {
            line.push_str(" NOT NULL");
        } else if engine != Engine::MySql {
            // MySQL spells the default nullable; saying NULL is noise there.
            line.push_str(" NULL");
        }
        if !self.default.trim().is_empty() {
            line.push_str(&format!(" DEFAULT {}", self.default.trim()));
        }
        if !self.comment.trim().is_empty() && engine == Engine::MySql {
            line.push_str(&format!(
                " COMMENT {}",
                quote::literal(engine, self.comment.trim())
            ));
        }
        line
    }
}

/// A foreign key to add with the table.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct ForeignKey {
    pub name: String,
    pub columns: Vec<String>,
    /// Where it points, already split into its parts.
    pub references: Vec<String>,
    pub referenced_columns: Vec<String>,
    pub on_delete: String,
    pub on_update: String,
}

/// A table as the Create Table wizard describes it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct TableSpec {
    pub columns: Vec<ColumnSpec>,
    /// Constraint names, when the engine wants one and the user gave it.
    pub unique_names: Vec<String>,
    pub primary_key_name: String,
    pub foreign_keys: Vec<ForeignKey>,
}

/// `CREATE TABLE` for the engine.
///
/// The engines disagree about almost everything here: MySQL puts indexes and
/// comments inside the statement and takes no schema, SQL Server will not
/// create a table without a schema, and only PostgreSQL adds `IF NOT EXISTS`
/// and a constraint block in one statement. Those differences are the point of
/// this function rather than something to paper over.
pub fn create_table(
    engine: Engine,
    database: &str,
    schema: &str,
    name: &str,
    spec: &TableSpec,
) -> Result<String, String> {
    if spec.columns.is_empty() {
        return Err("A table needs at least one column.".to_owned());
    }
    let mut body: Vec<String> = spec
        .columns
        .iter()
        .map(|column| format!("  {}", column.column_line(engine)))
        .collect();

    // The key columns, in the order they were given.
    let key: Vec<&ColumnSpec> = spec
        .columns
        .iter()
        .filter(|column| column.primary_key)
        .collect();
    if !key.is_empty() {
        body.push(format!(
            "  PRIMARY KEY ({})",
            join_idents(engine, key.iter().map(|c| c.name.as_str()))
        ));
    }
    let uniques: Vec<&ColumnSpec> = spec
        .columns
        .iter()
        .filter(|column| column.unique && !column.primary_key)
        .collect();
    if !uniques.is_empty() {
        body.push(format!(
            "  UNIQUE ({})",
            join_idents(engine, uniques.iter().map(|c| c.name.as_str()))
        ));
    }
    for (index, key) in spec.foreign_keys.iter().enumerate() {
        body.push(foreign_key_clause(engine, key, index));
    }

    let exists = match engine {
        // MySQL's CREATE TABLE IF NOT EXISTS is silent about a different shape,
        // so it is left off and the wizard says the name does not exist.
        Engine::Postgres => "IF NOT EXISTS ",
        _ => "",
    };
    let engine_note = match engine {
        Engine::MySql => " ENGINE=InnoDB".to_owned(),
        _ => String::new(),
    };
    Ok(format!(
        "CREATE TABLE {exists}{}{} (\n{}\n){engine_note};",
        quote::table(engine, database, schema, name),
        suffix(engine, spec),
        body.join(",\n")
    ))
}

/// What follows the table's name in CREATE TABLE, which is where each engine
/// keeps its options.
fn suffix(engine: Engine, spec: &TableSpec) -> String {
    match engine {
        Engine::Postgres => {
            let mut parts = Vec::new();
            // A named key is only needed when the columns did not carry it.
            if !spec.primary_key_name.trim().is_empty()
                && !spec.columns.iter().any(|c| c.primary_key)
            {
                parts.push(format!(
                    "CONSTRAINT {} PRIMARY KEY ({})",
                    quote::ident(engine, spec.primary_key_name.trim()),
                    join_idents(engine, spec.columns.iter().map(|c| c.name.as_str()))
                ));
            }
            if parts.is_empty() {
                String::new()
            } else {
                format!(" ({})", parts.join(", "))
            }
        }
        _ => String::new(),
    }
}

pub(super) fn foreign_key_clause(engine: Engine, key: &ForeignKey, index: usize) -> String {
    let mut clause = String::from("  ");
    if key.name.trim().is_empty() {
        // Every engine will name one; leaving it to them keeps the DDL short.
        if engine == Engine::MySql {
            clause.push_str("CONSTRAINT ");
        }
    } else {
        clause.push_str(&format!(
            "CONSTRAINT {} ",
            quote::ident(engine, key.name.trim())
        ));
    }
    clause.push_str(&format!(
        "FOREIGN KEY ({}) REFERENCES {} ({})",
        join_idents(engine, key.columns.iter().map(String::as_str)),
        quote::qualified(
            engine,
            &key.references
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        ),
        join_idents(engine, key.referenced_columns.iter().map(String::as_str)),
    ));
    if !key.on_delete.trim().is_empty() {
        clause.push_str(&format!(" ON DELETE {}", key.on_delete.trim()));
    }
    if !key.on_update.trim().is_empty() {
        clause.push_str(&format!(" ON UPDATE {}", key.on_update.trim()));
    }
    let _ = index;
    clause
}

/// `ALTER TABLE … RENAME`, which is spelled three different ways.
pub fn rename_table(engine: Engine, database: &str, schema: &str, name: &str, to: &str) -> String {
    let current = quote::table(engine, database, schema, name);
    let next = quote::ident(engine, to);
    match engine {
        // SQL Server has no ALTER TABLE … RENAME TO. The object name is a
        // string here, and it is the quoted name — brackets inside a string
        // literal would be taken as part of the name.
        Engine::SqlServer => format!("EXEC sp_rename N'{current}', N'{to}';"),
        _ => format!("ALTER TABLE {current} RENAME TO {next};"),
    }
}

/// `DROP TABLE`, with the guard where the engine has one.
pub fn drop_table(
    engine: Engine,
    database: &str,
    schema: &str,
    name: &str,
    if_exists: bool,
) -> String {
    let exists = if if_exists && engine != Engine::MySql {
        "IF EXISTS "
    } else {
        ""
    };
    format!(
        "DROP TABLE {exists}{};",
        quote::table(engine, database, schema, name)
    )
}

/// `TRUNCATE`, which on PostgreSQL needs the restart flag spelled out to mean
/// what it means on the other two.
pub fn truncate_table(
    engine: Engine,
    database: &str,
    schema: &str,
    name: &str,
    restart_identity: bool,
) -> String {
    let table = quote::table(engine, database, schema, name);
    match engine {
        Engine::Postgres if restart_identity => {
            format!("TRUNCATE TABLE {table} RESTART IDENTITY;")
        }
        _ => format!("TRUNCATE TABLE {table};"),
    }
}

pub(super) fn join_idents(engine: Engine, names: impl Iterator<Item = impl AsRef<str>>) -> String {
    names
        .map(|name| quote::ident(engine, name.as_ref()))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::ddl::outside_identifiers;

    fn spec() -> TableSpec {
        TableSpec {
            columns: vec![
                ColumnSpec {
                    name: "id".into(),
                    data_type: "INTEGER".into(),
                    nullable: false,
                    default: String::new(),
                    comment: String::new(),
                    primary_key: true,
                    unique: false,
                },
                ColumnSpec {
                    name: "note".into(),
                    data_type: "TEXT".into(),
                    nullable: true,
                    default: String::new(),
                    comment: String::new(),
                    primary_key: false,
                    unique: false,
                },
                ColumnSpec {
                    name: "status".into(),
                    data_type: "VARCHAR(20)".into(),
                    nullable: false,
                    default: "'new'".into(),
                    comment: String::new(),
                    primary_key: false,
                    unique: true,
                },
            ],
            unique_names: vec![],
            primary_key_name: String::new(),
            foreign_keys: vec![],
        }
    }

    #[test]
    fn creates_a_table_for_each_engine() {
        assert_eq!(
            create_table(Engine::Postgres, "shop", "public", "orders", &spec()).unwrap(),
            "CREATE TABLE IF NOT EXISTS \"public\".\"orders\" (\n  \
             \"id\" INTEGER NOT NULL,\n  \
             \"note\" TEXT NULL,\n  \
             \"status\" VARCHAR(20) NOT NULL DEFAULT 'new',\n  \
             PRIMARY KEY (\"id\"),\n  \
             UNIQUE (\"status\")\n);"
        );
        // MySQL: no schema, no IF NOT EXISTS, an engine, and no NULL on a
        // nullable column because that is already the default.
        assert_eq!(
            create_table(Engine::MySql, "shop", "", "orders", &spec()).unwrap(),
            "CREATE TABLE `shop`.`orders` (\n  \
             `id` INTEGER NOT NULL,\n  \
             `note` TEXT,\n  \
             `status` VARCHAR(20) NOT NULL DEFAULT 'new',\n  \
             PRIMARY KEY (`id`),\n  \
             UNIQUE (`status`)\n) ENGINE=InnoDB;"
        );
        assert_eq!(
            create_table(Engine::SqlServer, "shop", "dbo", "orders", &spec()).unwrap(),
            "CREATE TABLE [shop].[dbo].[orders] (\n  \
             [id] INTEGER NOT NULL,\n  \
             [note] TEXT NULL,\n  \
             [status] VARCHAR(20) NOT NULL DEFAULT 'new',\n  \
             PRIMARY KEY ([id]),\n  \
             UNIQUE ([status])\n);"
        );
    }

    #[test]
    fn quotes_awkward_names_rather_than_escaping_them() {
        let mut awkward = spec();
        awkward.columns[0].name = "order\"; DROP TABLE x; --".into();
        awkward.columns[0].primary_key = false;
        let sql = create_table(Engine::Postgres, "shop", "public", "Odd Name", &awkward).unwrap();
        // One identifier, doubled inside, and the semicolon never ends anything.
        assert!(sql.contains("\"order\"\"; DROP TABLE x; --\" INTEGER NOT NULL"));
        assert!(sql.contains("\"Odd Name\""));
        // A semicolon inside the quoted name is a character, not a statement
        // break: with the name taken out, one `;` is left, the terminator.
        let without_names = outside_identifiers(&sql);
        assert_eq!(
            without_names.matches(';').count(),
            1,
            "a name ended the statement: {without_names}"
        );

        let reserved = TableSpec {
            columns: vec![ColumnSpec {
                name: "select".into(),
                data_type: "INTEGER".into(),
                nullable: true,
                ..ColumnSpec::default()
            }],
            ..TableSpec::default()
        };
        for (engine, expected) in [
            (Engine::Postgres, "\"select\" INTEGER NULL"),
            (Engine::MySql, "`select` INTEGER"),
            (Engine::SqlServer, "[select] INTEGER NULL"),
        ] {
            assert!(
                create_table(engine, "shop", "public", "t", &reserved)
                    .unwrap()
                    .contains(expected)
            );
        }
    }

    #[test]
    fn keeps_unicode_names_whole() {
        let mut unicode = spec();
        unicode.columns[1].name = "ünïcödé".into();
        let sql = create_table(Engine::Postgres, "shop", "public", "t", &unicode).unwrap();
        assert!(sql.contains("\"ünïcödé\" TEXT NULL"));
    }

    #[test]
    fn writes_a_foreign_key_with_its_actions() {
        let mut with_key = spec();
        with_key.foreign_keys.push(ForeignKey {
            name: "orders_customer".into(),
            columns: vec!["customer_id".into()],
            references: vec!["public".into(), "customers".into()],
            referenced_columns: vec!["id".into()],
            on_delete: "CASCADE".into(),
            on_update: String::new(),
        });
        let pg = create_table(Engine::Postgres, "shop", "public", "orders", &with_key).unwrap();
        assert!(pg.contains(
            "CONSTRAINT \"orders_customer\" FOREIGN KEY (\"customer_id\") \
             REFERENCES \"public\".\"customers\" (\"id\") ON DELETE CASCADE"
        ));
        let my = create_table(Engine::MySql, "shop", "", "orders", &with_key).unwrap();
        assert!(my.contains(
            "CONSTRAINT `orders_customer` FOREIGN KEY (`customer_id`) \
             REFERENCES `public`.`customers` (`id`) ON DELETE CASCADE"
        ));
    }

    #[test]
    fn refuses_a_table_with_no_columns() {
        for engine in [Engine::Postgres, Engine::MySql, Engine::SqlServer] {
            let error = create_table(engine, "shop", "public", "t", &TableSpec::default());
            assert!(error.is_err(), "{engine:?} built a table with no columns");
        }
    }

    #[test]
    fn removes_a_table_with_the_guard_each_engine_offers() {
        assert_eq!(
            drop_table(Engine::Postgres, "shop", "public", "orders", true),
            "DROP TABLE IF EXISTS \"public\".\"orders\";"
        );
        // MySQL has no IF EXISTS on DROP TABLE in the versions this app supports.
        assert_eq!(
            drop_table(Engine::MySql, "shop", "", "orders", true),
            "DROP TABLE `shop`.`orders`;"
        );
        assert_eq!(
            drop_table(Engine::SqlServer, "shop", "dbo", "orders", false),
            "DROP TABLE [shop].[dbo].[orders];"
        );
    }

    #[test]
    fn renames_the_way_each_engine_does() {
        assert_eq!(
            rename_table(
                Engine::Postgres,
                "shop",
                "public",
                "orders",
                "archived_orders"
            ),
            "ALTER TABLE \"public\".\"orders\" RENAME TO \"archived_orders\";"
        );
        assert_eq!(
            rename_table(Engine::MySql, "shop", "", "orders", "archived_orders"),
            "ALTER TABLE `shop`.`orders` RENAME TO `archived_orders`;"
        );
        // SQL Server has no RENAME TO; it renames through a stored procedure,
        // and the name goes as a literal because that is what it takes.
        assert_eq!(
            rename_table(
                Engine::SqlServer,
                "shop",
                "dbo",
                "orders",
                "archived orders"
            ),
            "EXEC sp_rename N'[shop].[dbo].[orders]', N'archived orders';"
        );
    }

    #[test]
    fn truncates_and_only_postgres_needs_the_identity_flag() {
        assert_eq!(
            truncate_table(Engine::Postgres, "shop", "public", "orders", false),
            "TRUNCATE TABLE \"public\".\"orders\";"
        );
        assert_eq!(
            truncate_table(Engine::Postgres, "shop", "public", "orders", true),
            "TRUNCATE TABLE \"public\".\"orders\" RESTART IDENTITY;"
        );
        assert_eq!(
            truncate_table(Engine::MySql, "shop", "", "orders", true),
            "TRUNCATE TABLE `shop`.`orders`;"
        );
    }
}

//! `CREATE TABLE`, and the actions that remove or rename one, per engine.

use super::super::{Engine, quote};

/// A column as the designer describes it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    fn column_line(&self, engine: Engine) -> String {
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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

fn foreign_key_clause(engine: Engine, key: &ForeignKey, index: usize) -> String {
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

fn join_idents(engine: Engine, names: impl Iterator<Item = impl AsRef<str>>) -> String {
    names
        .map(|name| quote::ident(engine, name.as_ref()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The statement text with every quoted identifier taken out, so a semicolon
/// inside a name is not counted as a statement break. Asserting on a statement
/// that quotes an awkward name needs this: the name is free to contain the
/// character the check is looking for.
#[cfg(test)]
fn outside_identifiers(sql: &str) -> String {
    let mut out = String::new();
    let mut rest = sql;
    while let Some(open) = rest.find(['"', '`', '[']) {
        out.push_str(&rest[..open]);
        let close = if rest[open..].starts_with('[') {
            "]"
        } else {
            &rest[open..open + 1]
        };
        match rest[open + 1..].find(close) {
            Some(at) => rest = &rest[open + 1 + at + 1..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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

/// A change to a table that already exists.
///
/// One of these is one thing the person asked for, which may be more than one
/// statement: SQL Server changes a column's type in a statement of its own,
/// and cannot do it in the same one that changes nullability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    AddColumn(ColumnSpec),
    DropColumn {
        name: String,
    },
    RenameColumn {
        from: String,
        to: String,
        /// Carried so a rename and a re-type stay one action.
        spec: ColumnSpec,
    },
    SetType {
        name: String,
        data_type: String,
    },
    SetNullable {
        name: String,
        nullable: bool,
        /// Carried because MySQL and SQL Server restate the column to change
        /// nullability, and the restatement needs its type.
        data_type: String,
    },
    /// An empty default drops it.
    SetDefault {
        name: String,
        default: String,
    },
    SetComment {
        name: String,
        comment: String,
        /// Carried for the same reason as `SetNullable`.
        data_type: String,
    },
    /// PostgreSQL and SQL Server only; MySQL has no column order to speak of
    /// outside `FIRST`/`AFTER`, which is a separate change.
    MoveColumn {
        name: String,
        after: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexSpec {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyChange {
    SetPrimaryKey {
        name: String,
        columns: Vec<String>,
    },
    /// Carries the constraint's name: all three engines drop it by name, and
    /// a guessed name would fail with a constraint that does not exist.
    DropPrimaryKey {
        name: String,
    },
    AddForeignKey(ForeignKey),
    DropForeignKey {
        name: String,
    },
    AddCheck {
        name: String,
        expression: String,
    },
    DropCheck {
        name: String,
    },
}

/// `ALTER TABLE` statements for one change, in the order they must run.
///
/// A `Vec` because SQL Server needs `ALTER COLUMN` on its own, and because
/// dropping a key and adding one of a different shape are two statements.
pub fn alter_column(
    engine: Engine,
    database: &str,
    schema: &str,
    table: &str,
    change: &Change,
) -> Vec<String> {
    let target = quote::table(engine, database, schema, table);
    match change {
        Change::AddColumn(column) => vec![format!(
            "ALTER TABLE {target} ADD COLUMN {};",
            column.column_line(engine)
        )],
        Change::DropColumn { name } => {
            vec![format!(
                "ALTER TABLE {target} DROP COLUMN {};",
                quote::ident(engine, name)
            )]
        }
        Change::RenameColumn { from, to, spec } => match engine {
            // MySQL renames by restating the whole column.
            Engine::MySql => vec![format!(
                "ALTER TABLE {target} CHANGE COLUMN {} {};",
                quote::ident(engine, from),
                spec.column_line(engine)
            )],
            // SQL Server has no column rename in DDL; it goes through a
            // procedure and says which kind of object it is renaming.
            Engine::SqlServer => vec![
                format!(
                    "EXEC sp_rename N'{}.{}.{}.{}', N'{}', N'COLUMN';",
                    quote::ident(engine, database),
                    quote::ident(engine, schema),
                    quote::ident(engine, table),
                    quote::ident(engine, from),
                    if to.is_empty() { from.as_str() } else { to }
                ),
                // The type still needs restating, since sp_rename does not
                // touch it.
                format!(
                    "ALTER TABLE {target} ALTER COLUMN {};",
                    spec.column_line(engine)
                ),
            ],
            Engine::Postgres => vec![format!(
                "ALTER TABLE {target} RENAME COLUMN {} TO {};",
                quote::ident(engine, from),
                quote::ident(engine, to)
            )],
        },
        Change::SetType { name, data_type } => match engine {
            // Postgres will not cast implicitly; the USING clause says how,
            // and a cast that cannot work is the server's to refuse.
            Engine::Postgres => vec![format!(
                "ALTER TABLE {target} ALTER COLUMN {} TYPE {} USING {}::{};",
                quote::ident(engine, name),
                data_type.trim(),
                quote::ident(engine, name),
                data_type.trim()
            )],
            _ => vec![format!(
                "ALTER TABLE {target} ALTER COLUMN {} {};",
                quote::ident(engine, name),
                data_type.trim()
            )],
        },
        Change::SetNullable {
            name,
            nullable,
            data_type,
        } => {
            let column = quote::ident(engine, name);
            let word = if *nullable {
                "DROP NOT NULL"
            } else {
                "SET NOT NULL"
            };
            match engine {
                // MySQL has no SET NOT NULL: nullability is part of the
                // column's restatement.
                Engine::MySql => vec![format!(
                    "ALTER TABLE {target} MODIFY COLUMN {column} {} {};",
                    data_type.trim(),
                    if *nullable { "NULL" } else { "NOT NULL" }
                )],
                Engine::Postgres => vec![format!(
                    "ALTER TABLE {target} ALTER COLUMN {column} {word};"
                )],
                // SQL Server needs a type with every nullability change.
                Engine::SqlServer => vec![format!(
                    "ALTER TABLE {target} ALTER COLUMN {column} {} {};",
                    data_type.trim(),
                    if *nullable { "NULL" } else { "NOT NULL" }
                )],
            }
        }
        Change::SetDefault { name, default } => {
            let column = quote::ident(engine, name);
            if default.trim().is_empty() {
                vec![match engine {
                    Engine::MySql => {
                        format!("ALTER TABLE {target} ALTER COLUMN {column} DROP DEFAULT;")
                    }
                    _ => format!("ALTER TABLE {target} ALTER COLUMN {column} DROP DEFAULT;"),
                }]
            } else {
                vec![format!(
                    "ALTER TABLE {target} ALTER COLUMN {column} SET DEFAULT {};",
                    default.trim()
                )]
            }
        }
        Change::SetComment {
            name,
            comment,
            data_type,
        } => match engine {
            // Only MySQL keeps a comment on the column itself.
            Engine::MySql => vec![format!(
                "ALTER TABLE {target} MODIFY COLUMN {} {} COMMENT {};",
                quote::ident(engine, name),
                data_type.trim(),
                if comment.trim().is_empty() {
                    "''".to_owned()
                } else {
                    quote::literal(engine, comment.trim())
                }
            )],
            // PostgreSQL comments are separate statements on the object.
            Engine::Postgres => {
                let column = quote::ident(engine, name);
                vec![format!(
                    "COMMENT ON COLUMN {target}.{column} IS {};",
                    if comment.trim().is_empty() {
                        "NULL".to_owned()
                    } else {
                        quote::literal(engine, comment.trim())
                    }
                )]
            }
            // SQL Server has no column comment; a description is an extended
            // property, and changing one means dropping the old first.
            _ => {
                let described = if comment.trim().is_empty() {
                    "NULL".to_owned()
                } else {
                    quote::literal(engine, comment.trim())
                };
                vec![format!(
                    "EXEC sp_updateextendedproperty N'MS_Description', {}, N'SCHEMA', N'{database}', \
                     N'TABLE', N'{table}', N'COLUMN', N'{name}';",
                    described
                )]
            }
        },
        Change::MoveColumn { name, after } => match engine {
            // Column order is a MySQL concept in DDL.
            Engine::MySql => {
                let column = quote::ident(engine, name);
                let position = match after {
                    Some(previous) => format!(" AFTER {}", quote::ident(engine, previous)),
                    None => " FIRST".to_owned(),
                };
                vec![format!(
                    "ALTER TABLE {target} MODIFY COLUMN {column}{position};"
                )]
            }
            _ => Vec::new(),
        },
    }
}

/// Statements for an index, which lives in a different place on each engine.
pub fn create_index(
    engine: Engine,
    database: &str,
    schema: &str,
    table: &str,
    index: &IndexSpec,
) -> String {
    let name = quote::ident(engine, &index.name);
    let columns = join_idents(engine, index.columns.iter().map(String::as_str));
    match engine {
        // MySQL keeps indexes inside the table's own statement.
        Engine::MySql => format!(
            "ALTER TABLE {} ADD {}INDEX {} ({});",
            quote::table(engine, database, schema, table),
            if index.unique { "UNIQUE " } else { "" },
            name,
            columns
        ),
        _ => format!(
            "CREATE {}INDEX {name} ON {} ({columns});",
            if index.unique { "UNIQUE " } else { "" },
            quote::table(engine, database, schema, table)
        ),
    }
}

pub fn drop_index(
    engine: Engine,
    database: &str,
    schema: &str,
    table: &str,
    index: &str,
) -> String {
    let name = quote::ident(engine, index);
    match engine {
        Engine::MySql => format!(
            "ALTER TABLE {} DROP INDEX {name};",
            quote::table(engine, database, schema, table)
        ),
        // PostgreSQL and SQL Server drop an index without naming the table.
        _ => format!("DROP INDEX {name};"),
    }
}

/// Statements for a key or constraint.
pub fn alter_key(
    engine: Engine,
    database: &str,
    schema: &str,
    table: &str,
    change: &KeyChange,
) -> Vec<String> {
    let target = quote::table(engine, database, schema, table);
    match change {
        KeyChange::SetPrimaryKey { name, columns } => {
            let columns = join_idents(engine, columns.iter().map(String::as_str));
            if engine == Engine::MySql && !name.trim().is_empty() {
                return vec![format!(
                    "ALTER TABLE {target} ADD CONSTRAINT {} PRIMARY KEY ({columns});",
                    quote::ident(engine, name.trim())
                )];
            }
            vec![format!("ALTER TABLE {target} ADD PRIMARY KEY ({columns});")]
        }
        KeyChange::DropPrimaryKey { name } => vec![format!(
            "ALTER TABLE {target} DROP CONSTRAINT {};",
            quote::ident(engine, name)
        )],
        KeyChange::AddForeignKey(key) => vec![format!(
            "ALTER TABLE {target} ADD {};",
            foreign_key_clause(engine, key, 0).trim_start()
        )],
        // Every engine drops a constraint by name.
        KeyChange::DropForeignKey { name } | KeyChange::DropCheck { name } => {
            vec![format!(
                "ALTER TABLE {target} DROP CONSTRAINT {};",
                quote::ident(engine, name)
            )]
        }
        KeyChange::AddCheck { name, expression } => vec![format!(
            "ALTER TABLE {target} ADD CONSTRAINT {} CHECK ({});",
            quote::ident(engine, name),
            expression.trim()
        )],
    }
}

#[cfg(test)]
mod alter_tests {
    use super::*;

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
            statements[0]
                .contains("sp_rename N'[shop].[dbo].[orders].[note]', N'summary', N'COLUMN';")
        );
        assert!(
            statements[1].starts_with("ALTER TABLE [shop].[dbo].[orders] ALTER COLUMN [summary]")
        );
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
            ["ALTER TABLE `shop`.`orders` ALTER COLUMN `total` DECIMAL(10,2);"]
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
            [
                "ALTER TABLE `shop`.`orders` MODIFY COLUMN `status` VARCHAR(20) COMMENT 'where it is';"
            ]
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
            [
                "ALTER TABLE [shop].[dbo].[orders] ADD CONSTRAINT [total_positive] CHECK (total >= 0);"
            ]
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
}

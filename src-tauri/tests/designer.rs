//! The table designer's changes, applied to real databases and read back.
//!
//! `drivers.rs` proves the common path. This file covers every other change
//! the designer can stage, and above all that a change keeps what it was not
//! asked to change: retyping a column must not drop its NOT NULL or default.
//!
//! `#[ignore]`d like the other integration tests; run them with
//! `npm run test:integration`.

use thabi_studio_lib::database::ddl::{
    Change, ColumnSpec, KeyChange, TableAction, TablePlan, TableSpec, action_script, create_table,
    current, drop_table, spec_from_structure, table_script_with,
};
use thabi_studio_lib::database::metadata::ObjectStructure;
use thabi_studio_lib::database::{ConnectParams, Engine, ObjectKind, Session, TlsMode};

fn env(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} is not set; run npm run test:integration"))
}

async fn open(engine: Engine) -> Session {
    open_as(engine, None).await
}

/// MySQL's test login may only use its own database; making and dropping
/// another one needs root, which the container's password also opens.
async fn open_as(engine: Engine, user: Option<&str>) -> Session {
    let (port, username, database) = match engine {
        Engine::Postgres => (env("THABI_IT_PG_PORT"), "thabi", "thabi_it"),
        Engine::MySql => (env("THABI_IT_MYSQL_PORT"), "thabi", "thabi_it"),
        Engine::SqlServer => (env("THABI_IT_MSSQL_PORT"), "sa", "master"),
    };
    let params = ConnectParams {
        engine,
        host: "127.0.0.1".into(),
        port: port.parse().unwrap(),
        database: Some(database.into()),
        username: user.unwrap_or(username).into(),
        password: Some(env("THABI_IT_PASSWORD")),
        tls: match engine {
            Engine::Postgres => TlsMode::Off,
            _ => TlsMode::EncryptOnly,
        },
        ca_path: None,
        client_cert: None,
        client_key: None,
        ssh: None,
        connect_to: None,
        read_only: false,
        startup_commands: Vec::new(),
    };
    Session::connect(&params)
        .await
        .unwrap_or_else(|e| panic!("{engine:?} connect failed: {}", e.message))
}

fn place(engine: Engine) -> (&'static str, &'static str) {
    match engine {
        Engine::MySql => ("thabi_it", ""),
        Engine::Postgres => ("thabi_it", "public"),
        Engine::SqlServer => ("master", "dbo"),
    }
}

const TABLE: &str = "thabi designer";

struct Fixture {
    engine: Engine,
    session: Session,
    db: &'static str,
    schema: &'static str,
}

impl Fixture {
    async fn new(engine: Engine) -> Self {
        let (db, schema) = place(engine);
        let mut fixture = Self {
            engine,
            session: open(engine).await,
            db,
            schema,
        };
        for leftover in [TABLE, "thabi designer copy"] {
            fixture
                .run(&drop_table(engine, db, schema, leftover, true))
                .await;
        }
        let text = match engine {
            Engine::SqlServer => "NVARCHAR(20)",
            _ => "VARCHAR(20)",
        };
        let spec = TableSpec {
            columns: vec![
                ColumnSpec {
                    name: "id".into(),
                    data_type: "INTEGER".into(),
                    nullable: false,
                    primary_key: true,
                    ..Default::default()
                },
                ColumnSpec {
                    name: "status".into(),
                    data_type: text.into(),
                    nullable: false,
                    default: "'new'".into(),
                    ..Default::default()
                },
                ColumnSpec {
                    name: "note".into(),
                    data_type: text.into(),
                    nullable: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let created = create_table(engine, db, schema, TABLE, &spec).expect("create table");
        fixture.run(&created).await;
        fixture
    }

    async fn run(&mut self, sql: &str) {
        let engine = self.engine;
        self.session
            .execute(sql, 1)
            .await
            .unwrap_or_else(|e| panic!("{engine:?} refused {sql}\n{} {:?}", e.message, e.detail));
    }

    /// Builds the script the way the app does: after reading the table.
    async fn apply(&mut self, plan: TablePlan) {
        let now = current::read(&mut self.session, self.engine, self.db, self.schema, TABLE)
            .await
            .expect("read the table's columns");
        for statement in table_script_with(self.engine, self.db, self.schema, TABLE, &plan, &now) {
            self.run(&statement).await;
        }
    }

    async fn describe(&mut self, table: &str) -> ObjectStructure {
        self.session
            .describe(self.db, self.schema, table, ObjectKind::Table)
            .await
            .unwrap_or_else(|e| panic!("{:?} describe {table}: {}", self.engine, e.message))
    }
}

fn column<'a>(
    structure: &'a ObjectStructure,
    name: &str,
) -> &'a thabi_studio_lib::database::metadata::ColumnInfo {
    structure
        .columns
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no column {name} in {:?}", structure.columns))
}

fn columns(plan: Vec<Change>) -> TablePlan {
    TablePlan {
        columns: plan,
        ..Default::default()
    }
}

async fn keeps_what_it_does_not_change(engine: Engine) {
    let mut t = Fixture::new(engine).await;
    let wider = match engine {
        Engine::SqlServer => "NVARCHAR(40)",
        _ => "VARCHAR(40)",
    };

    // Retyping keeps NOT NULL and the default.
    t.apply(columns(vec![Change::SetType {
        name: "status".into(),
        data_type: wider.into(),
    }]))
    .await;
    let after = t.describe(TABLE).await;
    let status = column(&after, "status");
    assert!(!status.nullable, "{engine:?} retype lost NOT NULL");
    assert!(
        status.default.as_deref().is_some_and(|d| d.contains("new")),
        "{engine:?} retype lost the default: {:?}",
        status.default
    );

    // Renaming keeps them too.
    t.apply(columns(vec![Change::RenameColumn {
        from: "status".into(),
        to: "state".into(),
        spec: ColumnSpec {
            name: "state".into(),
            data_type: wider.into(),
            nullable: false,
            ..Default::default()
        },
    }]))
    .await;
    let after = t.describe(TABLE).await;
    let state = column(&after, "state");
    assert!(!state.nullable, "{engine:?} rename lost NOT NULL");
    assert!(
        state.default.as_deref().is_some_and(|d| d.contains("new")),
        "{engine:?} rename lost the default: {:?}",
        state.default
    );

    // Making it nullable keeps the default.
    t.apply(columns(vec![Change::SetNullable {
        name: "state".into(),
        nullable: true,
        data_type: wider.into(),
    }]))
    .await;
    let after = t.describe(TABLE).await;
    let state = column(&after, "state");
    assert!(state.nullable, "{engine:?} did not make it nullable");
    assert!(
        state.default.as_deref().is_some_and(|d| d.contains("new")),
        "{engine:?} nullability change lost the default: {:?}",
        state.default
    );
}

async fn sets_and_drops_defaults_and_comments(engine: Engine) {
    let mut t = Fixture::new(engine).await;
    t.apply(columns(vec![Change::SetDefault {
        name: "note".into(),
        default: "'hello'".into(),
    }]))
    .await;
    let after = t.describe(TABLE).await;
    assert!(
        column(&after, "note")
            .default
            .as_deref()
            .is_some_and(|d| d.contains("hello")),
        "{engine:?} did not set the default"
    );
    // Replacing a default that exists, then dropping it.
    t.apply(columns(vec![Change::SetDefault {
        name: "note".into(),
        default: "'again'".into(),
    }]))
    .await;
    t.apply(columns(vec![Change::SetDefault {
        name: "note".into(),
        default: String::new(),
    }]))
    .await;
    let after = t.describe(TABLE).await;
    assert_eq!(
        column(&after, "note").default,
        None,
        "{engine:?} did not drop the default"
    );

    // A comment, twice: the second replaces the first, wherever it lives.
    let text = match engine {
        Engine::SqlServer => "NVARCHAR(20)",
        _ => "VARCHAR(20)",
    };
    for comment in ["first", "it's second"] {
        t.apply(columns(vec![Change::SetComment {
            name: "status".into(),
            comment: comment.into(),
            data_type: text.into(),
        }]))
        .await;
    }
    let after = t.describe(TABLE).await;
    let status = column(&after, "status");
    assert!(!status.nullable, "{engine:?} comment lost NOT NULL");
    assert!(
        status.default.as_deref().is_some_and(|d| d.contains("new")),
        "{engine:?} comment lost the default"
    );
}

async fn drops_columns_keys_and_checks(engine: Engine) {
    let mut t = Fixture::new(engine).await;
    t.apply(TablePlan {
        keys: vec![KeyChange::AddCheck {
            name: "designer_note_check".into(),
            expression: "note <> ''".into(),
        }],
        ..Default::default()
    })
    .await;
    t.apply(TablePlan {
        keys: vec![KeyChange::DropCheck {
            name: "designer_note_check".into(),
        }],
        ..Default::default()
    })
    .await;
    // The check is gone if an empty note is accepted.
    let insert = format!(
        "INSERT INTO {} (id, status, note) VALUES (1, 'x', '')",
        thabi_studio_lib::database::quote::table(engine, t.db, t.schema, TABLE)
    );
    t.run(&insert).await;

    t.apply(columns(vec![Change::DropColumn {
        name: "note".into(),
    }]))
    .await;
    let after = t.describe(TABLE).await;
    assert!(
        after.columns.iter().all(|c| c.name != "note"),
        "{engine:?} kept the dropped column"
    );

    // The primary key, dropped by the name the server gave it, then set again.
    let pk_name = after
        .indexes
        .as_ref()
        .and_then(|indexes| indexes.iter().find(|i| i.primary))
        .map(|i| i.name.clone())
        .expect("a primary key");
    t.apply(TablePlan {
        keys: vec![KeyChange::DropPrimaryKey { name: pk_name }],
        ..Default::default()
    })
    .await;
    let after = t.describe(TABLE).await;
    assert!(
        after.columns.iter().all(|c| c.primary_key.is_none()),
        "{engine:?} kept the primary key"
    );
    t.apply(TablePlan {
        keys: vec![KeyChange::SetPrimaryKey {
            name: "designer_pk".into(),
            columns: vec!["id".into()],
        }],
        ..Default::default()
    })
    .await;
    let after = t.describe(TABLE).await;
    assert_eq!(column(&after, "id").primary_key, Some(1));
}

async fn reads_and_drops_constraints_by_name(engine: Engine) {
    let mut t = Fixture::new(engine).await;
    let (db, schema) = (t.db, t.schema);
    let parent = "thabi designer parent";
    t.run(&drop_table(engine, db, schema, parent, true)).await;
    let text = match engine {
        Engine::SqlServer => "NVARCHAR(20)",
        _ => "VARCHAR(20)",
    };
    let parent_spec = TableSpec {
        columns: vec![
            ColumnSpec {
                name: "id".into(),
                data_type: "INTEGER".into(),
                primary_key: true,
                ..Default::default()
            },
            ColumnSpec {
                name: "code".into(),
                data_type: text.into(),
                primary_key: true,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    t.run(&create_table(engine, db, schema, parent, &parent_spec).unwrap())
        .await;
    let references = match engine {
        Engine::MySql => vec![db.to_owned(), parent.to_owned()],
        _ => vec![schema.to_owned(), parent.to_owned()],
    };
    t.apply(TablePlan {
        keys: vec![
            KeyChange::AddForeignKey(thabi_studio_lib::database::ddl::ForeignKey {
                name: "designer_two_column_fk".into(),
                columns: vec!["id".into(), "status".into()],
                references,
                referenced_columns: vec!["id".into(), "code".into()],
                on_delete: "CASCADE".into(),
                on_update: "NO ACTION".into(),
            }),
            KeyChange::AddCheck {
                name: "designer_status_check".into(),
                expression: "status <> ''".into(),
            },
        ],
        ..Default::default()
    })
    .await;

    let after = t.describe(TABLE).await;
    let constraints = after.constraints.clone().expect("a table has constraints");
    let fk = constraints
        .iter()
        .find(|c| c.name == "designer_two_column_fk")
        .unwrap_or_else(|| panic!("{engine:?} did not report the key: {constraints:?}"));
    assert_eq!(fk.kind, "foreignKey");
    assert_eq!(fk.columns, ["id", "status"], "{engine:?}");
    assert_eq!(fk.referenced_table, parent);
    assert_eq!(fk.referenced_columns, ["id", "code"], "{engine:?}");
    assert_eq!(fk.on_delete, "CASCADE", "{engine:?}");
    let check = constraints
        .iter()
        .find(|c| c.name == "designer_status_check")
        .unwrap_or_else(|| panic!("{engine:?} did not report the check: {constraints:?}"));
    assert_eq!(check.kind, "check");
    assert!(check.definition.contains("status"), "{engine:?}: {check:?}");

    t.apply(TablePlan {
        keys: vec![
            KeyChange::DropForeignKey {
                name: fk.name.clone(),
            },
            KeyChange::DropCheck {
                name: check.name.clone(),
            },
        ],
        ..Default::default()
    })
    .await;
    let after = t.describe(TABLE).await;
    assert!(
        after.constraints.unwrap_or_default().is_empty(),
        "{engine:?} kept a dropped constraint"
    );
    t.run(&drop_table(engine, db, schema, parent, true)).await;
}

/// A backup's CREATE TABLE, run back, gives the same keys and checks: a
/// unique index on two columns stays one rule about the pair, and a
/// two-column foreign key keeps its columns and actions.
async fn backup_keeps_keys_and_checks(engine: Engine) {
    use thabi_studio_lib::database::dump::structure_statements;
    let mut t = Fixture::new(engine).await;
    let (db, schema) = (t.db, t.schema);
    let parent = "thabi backup parent";
    t.run(&drop_table(engine, db, schema, parent, true)).await;
    let text = match engine {
        Engine::SqlServer => "NVARCHAR(20)",
        _ => "VARCHAR(20)",
    };
    let parent_spec = TableSpec {
        columns: vec![
            ColumnSpec {
                name: "id".into(),
                data_type: "INTEGER".into(),
                primary_key: true,
                ..Default::default()
            },
            ColumnSpec {
                name: "code".into(),
                data_type: text.into(),
                primary_key: true,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    t.run(&create_table(engine, db, schema, parent, &parent_spec).unwrap())
        .await;
    let references = match engine {
        Engine::MySql => vec![db.to_owned(), parent.to_owned()],
        _ => vec![schema.to_owned(), parent.to_owned()],
    };
    t.apply(TablePlan {
        keys: vec![
            KeyChange::AddForeignKey(thabi_studio_lib::database::ddl::ForeignKey {
                name: "backup_two_column_fk".into(),
                columns: vec!["id".into(), "status".into()],
                references,
                referenced_columns: vec!["id".into(), "code".into()],
                on_delete: "CASCADE".into(),
                on_update: "NO ACTION".into(),
            }),
            KeyChange::AddCheck {
                name: "backup_status_check".into(),
                expression: "status <> ''".into(),
            },
        ],
        indexes: vec![thabi_studio_lib::database::ddl::IndexAction::Create(
            thabi_studio_lib::database::ddl::IndexSpec {
                name: "backup_pair_idx".into(),
                columns: vec!["status".into(), "note".into()],
                unique: true,
            },
        )],
        ..Default::default()
    })
    .await;

    let parent_before = t.describe(parent).await;
    let before = t.describe(TABLE).await;
    let mut script = structure_statements(engine, db, schema, parent, &parent_before);
    script.extend(structure_statements(engine, db, schema, TABLE, &before));
    t.run(&drop_table(engine, db, schema, TABLE, true)).await;
    t.run(&drop_table(engine, db, schema, parent, true)).await;
    for statement in script {
        // The file's comment lines are for people; each statement runs alone.
        let statement = statement.trim();
        if !statement.is_empty() && !statement.starts_with("--") {
            t.run(statement).await;
        }
    }

    let after = t.describe(TABLE).await;
    let pair = after
        .indexes
        .as_ref()
        .and_then(|indexes| indexes.iter().find(|i| i.name == "backup_pair_idx"))
        .unwrap_or_else(|| panic!("{engine:?} lost the unique index: {:?}", after.indexes));
    assert!(pair.unique);
    assert_eq!(pair.columns, ["status", "note"], "{engine:?}");
    let constraints = after.constraints.clone().unwrap_or_default();
    let fk = constraints
        .iter()
        .find(|c| c.kind == "foreignKey")
        .unwrap_or_else(|| panic!("{engine:?} lost the foreign key: {constraints:?}"));
    assert_eq!(fk.columns, ["id", "status"], "{engine:?}");
    assert_eq!(fk.on_delete, "CASCADE", "{engine:?}");
    assert!(
        constraints.iter().any(|c| c.kind == "check"),
        "{engine:?} lost the check: {constraints:?}"
    );

    // The pair rule, not two single-column rules: same status, other note.
    let quoted = |name: &str| thabi_studio_lib::database::quote::table(engine, db, schema, name);
    t.run(&format!(
        "INSERT INTO {} (id, code) VALUES (1, 'a'), (2, 'a')",
        quoted(parent)
    ))
    .await;
    t.run(&format!(
        "INSERT INTO {} (id, status, note) VALUES (1, 'a', 'x'), (2, 'a', 'y')",
        quoted(TABLE)
    ))
    .await;
    t.run(&drop_table(engine, db, schema, TABLE, true)).await;
    t.run(&drop_table(engine, db, schema, parent, true)).await;
}

/// Show CREATE: a view's definition runs back as written, and a table's
/// is a CREATE TABLE for that table.
async fn shows_create_statements(engine: Engine) {
    use thabi_studio_lib::database::definition::object_definition;
    let mut t = Fixture::new(engine).await;
    let (db, schema) = (t.db, t.schema);
    // SQL Server takes no database prefix on CREATE or DROP VIEW.
    let view = match engine {
        Engine::SqlServer => "[dbo].[thabi designer view]".to_owned(),
        _ => thabi_studio_lib::database::quote::table(engine, db, schema, "thabi designer view"),
    };
    let table = thabi_studio_lib::database::quote::table(engine, db, schema, TABLE);
    t.run(&format!("DROP VIEW IF EXISTS {view}")).await;
    t.run(&format!(
        "CREATE VIEW {view} AS SELECT id, status FROM {table} WHERE status <> 'gone'"
    ))
    .await;

    let shown = object_definition(
        &mut t.session,
        db,
        schema,
        "thabi designer view",
        ObjectKind::View,
    )
    .await
    .unwrap_or_else(|e| panic!("{engine:?} view definition: {}", e.message));
    assert!(shown.to_lowercase().contains("gone"), "{engine:?}: {shown}");
    t.run(&format!("DROP VIEW {view}")).await;
    t.run(shown.trim_end_matches(';')).await;
    let back = t.describe("thabi designer view").await;
    assert_eq!(back.columns.len(), 2, "{engine:?} view did not come back");
    t.run(&format!("DROP VIEW {view}")).await;

    let created = object_definition(&mut t.session, db, schema, TABLE, ObjectKind::Table)
        .await
        .expect("table definition");
    assert!(created.contains("CREATE TABLE"), "{engine:?}: {created}");
    assert!(!created.contains("DROP TABLE"), "{engine:?}: {created}");
}

/// Databases and schemas, made and removed with the statements the review
/// shows. Each statement runs on its own, as the command runs them.
async fn creates_and_drops_databases_and_schemas(engine: Engine) {
    use thabi_studio_lib::database::ddl::{ContainerAction, container_script};
    let mut t = Fixture::new(engine).await;
    let db = t.db;
    let run_all = async |t: &mut Fixture, action: ContainerAction| {
        for statement in container_script(engine, &action).expect("a script") {
            t.run(&statement).await;
        }
    };
    if engine != Engine::MySql {
        let schema = "thabi new schema";
        let _ = t
            .session
            .execute(
                &container_script(
                    engine,
                    &ContainerAction::DropSchema {
                        database: db.into(),
                        name: schema.into(),
                    },
                )
                .unwrap()[0],
                0,
            )
            .await;
        run_all(
            &mut t,
            ContainerAction::CreateSchema {
                database: db.into(),
                name: schema.into(),
            },
        )
        .await;
        let schemas = t
            .session
            .list_schemas(db)
            .await
            .unwrap()
            .unwrap_or_default();
        assert!(
            schemas.iter().any(|s| s == schema),
            "{engine:?}: {schemas:?}"
        );
        run_all(
            &mut t,
            ContainerAction::DropSchema {
                database: db.into(),
                name: schema.into(),
            },
        )
        .await;
    }
    let name = "thabi_new_database";
    if engine == Engine::MySql {
        t.session = open_as(engine, Some("root")).await;
    }
    let _ = t
        .session
        .execute(
            &format!(
                "DROP DATABASE IF EXISTS {}",
                thabi_studio_lib::database::quote::ident(engine, name)
            ),
            0,
        )
        .await;
    run_all(
        &mut t,
        ContainerAction::CreateDatabase { name: name.into() },
    )
    .await;
    let databases = t.session.list_databases().await.unwrap();
    assert!(
        databases.iter().any(|d| d == name),
        "{engine:?}: {databases:?}"
    );
    run_all(&mut t, ContainerAction::DropDatabase { name: name.into() }).await;
    let databases = t.session.list_databases().await.unwrap();
    assert!(!databases.iter().any(|d| d == name), "{engine:?} kept it");
}

/// Functions, procedures, triggers, sequences and types: listed, and their
/// definitions read and run back.
async fn lists_and_defines_routines(engine: Engine) {
    use thabi_studio_lib::database::routines::{list_routines, routine_definition};
    let mut t = Fixture::new(engine).await;
    let (db, schema) = (t.db, t.schema);
    // MySQL with binary logging lets only a privileged login create routines.
    if engine == Engine::MySql {
        t.session = open_as(engine, Some("root")).await;
    }
    let (setup, drops): (Vec<&str>, Vec<&str>) = match engine {
        Engine::Postgres => (
            vec![
                "CREATE OR REPLACE FUNCTION public.thabi_add(a int, b int) RETURNS int LANGUAGE sql AS $$ SELECT a + b $$",
                "CREATE OR REPLACE PROCEDURE public.thabi_proc() LANGUAGE sql AS $$ SELECT 1 $$",
                "CREATE OR REPLACE FUNCTION public.thabi_trg() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RETURN NEW; END $$",
                "CREATE TRIGGER thabi_trigger BEFORE INSERT ON public.\"thabi designer\" FOR EACH ROW EXECUTE FUNCTION public.thabi_trg()",
                "CREATE SEQUENCE public.thabi_seq START 5",
                "CREATE TYPE public.thabi_mood AS ENUM ('ok', 'it''s fine')",
            ],
            vec![
                "DROP FUNCTION IF EXISTS public.thabi_add(int, int)",
                "DROP PROCEDURE IF EXISTS public.thabi_proc()",
                "DROP FUNCTION IF EXISTS public.thabi_trg() CASCADE",
                "DROP SEQUENCE IF EXISTS public.thabi_seq",
                "DROP TYPE IF EXISTS public.thabi_mood",
            ],
        ),
        Engine::MySql => (
            vec![
                "CREATE FUNCTION thabi_add(a INT, b INT) RETURNS INT DETERMINISTIC RETURN a + b",
                "CREATE PROCEDURE thabi_proc() SELECT 1",
                "CREATE TRIGGER thabi_trigger BEFORE INSERT ON `thabi designer` FOR EACH ROW SET NEW.note = NEW.note",
            ],
            vec![
                "DROP FUNCTION IF EXISTS thabi_add",
                "DROP PROCEDURE IF EXISTS thabi_proc",
            ],
        ),
        Engine::SqlServer => (
            vec![
                "CREATE FUNCTION dbo.thabi_add(@a int, @b int) RETURNS int AS BEGIN RETURN @a + @b END",
                "CREATE PROCEDURE dbo.thabi_proc AS SELECT 1",
                "CREATE TRIGGER dbo.thabi_trigger ON dbo.[thabi designer] AFTER INSERT AS SET NOCOUNT ON",
                "CREATE SEQUENCE dbo.thabi_seq AS int START WITH 5",
                "CREATE TYPE dbo.thabi_code FROM nvarchar(10) NOT NULL",
            ],
            vec![
                "DROP FUNCTION IF EXISTS dbo.thabi_add",
                "DROP PROCEDURE IF EXISTS dbo.thabi_proc",
                "DROP SEQUENCE IF EXISTS dbo.thabi_seq",
                "DROP TYPE IF EXISTS dbo.thabi_code",
            ],
        ),
    };
    for statement in &drops {
        t.run(statement).await;
    }
    for statement in &setup {
        t.run(statement).await;
    }

    let listed = list_routines(&mut t.session, db, schema)
        .await
        .unwrap_or_else(|e| panic!("{engine:?} list: {} {:?}", e.message, e.detail));
    let find = |kind: &str, name: &str| {
        listed
            .iter()
            .find(|r| r.kind == kind && r.name == name)
            .cloned()
            .unwrap_or_else(|| panic!("{engine:?} did not list {kind} {name}: {listed:?}"))
    };
    let mut wanted = vec![
        ("function", "thabi_add"),
        ("procedure", "thabi_proc"),
        ("trigger", "thabi_trigger"),
    ];
    if engine != Engine::MySql {
        wanted.push(("sequence", "thabi_seq"));
    }
    match engine {
        Engine::Postgres => wanted.push(("type", "thabi_mood")),
        Engine::SqlServer => wanted.push(("type", "thabi_code")),
        Engine::MySql => {}
    }
    for (kind, name) in &wanted {
        let found = find(kind, name);
        let shown = routine_definition(&mut t.session, db, kind, &found.identity)
            .await
            .unwrap_or_else(|e| panic!("{engine:?} {kind} definition: {}", e.message));
        assert!(shown.contains(name), "{engine:?} {kind}: {shown}");
        if *kind == "trigger" {
            assert_eq!(found.table.as_deref(), Some(TABLE), "{engine:?}");
        }
    }

    // A definition runs back as it was given: the function, dropped and made
    // again from what Show Definition said.
    let add = find("function", "thabi_add");
    let shown = routine_definition(&mut t.session, db, "function", &add.identity)
        .await
        .unwrap();
    t.run(drops[0]).await;
    t.run(&shown).await;
    assert!(
        list_routines(&mut t.session, db, schema)
            .await
            .unwrap()
            .iter()
            .any(|r| r.name == "thabi_add")
    );
    for statement in &drops {
        t.run(statement).await;
    }
}

async fn duplicates_structure_and_drops(engine: Engine) {
    let mut t = Fixture::new(engine).await;
    let structure = t.describe(TABLE).await;
    let spec = spec_from_structure(&structure);
    let (db, schema) = (t.db, t.schema);
    for statement in action_script(
        engine,
        db,
        schema,
        TABLE,
        &TableAction::DuplicateStructure {
            to: "thabi designer copy".into(),
        },
        Some(&spec),
    )
    .expect("duplicate script")
    {
        t.run(&statement).await;
    }
    let copy = t.describe("thabi designer copy").await;
    let names: Vec<_> = copy.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["id", "status", "note"], "{engine:?} copy differs");
    assert!(!column(&copy, "status").nullable);

    for statement in action_script(
        engine,
        db,
        schema,
        "thabi designer copy",
        &TableAction::Drop { if_exists: false },
        None,
    )
    .expect("drop script")
    {
        t.run(&statement).await;
    }
    assert!(
        t.session
            .describe(db, schema, "thabi designer copy", ObjectKind::Table)
            .await
            .map(|s| s.columns.is_empty())
            .unwrap_or(true),
        "{engine:?} kept the dropped table"
    );
}

async fn moves_a_column_on_mysql() {
    let mut t = Fixture::new(Engine::MySql).await;
    t.apply(columns(vec![Change::MoveColumn {
        name: "note".into(),
        after: Some("id".into()),
    }]))
    .await;
    let after = t.describe(TABLE).await;
    let names: Vec<_> = after.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["id", "note", "status"]);
    let status = column(&after, "status");
    assert!(!status.nullable, "moving a column changed another one");
}

macro_rules! engine_tests {
    ($module:ident, $engine:expr) => {
        mod $module {
            use super::*;

            #[tokio::test]
            #[ignore]
            async fn keeps_what_a_change_does_not_touch() {
                keeps_what_it_does_not_change($engine).await;
            }

            #[tokio::test]
            #[ignore]
            async fn sets_and_drops_defaults_and_comments() {
                super::sets_and_drops_defaults_and_comments($engine).await;
            }

            #[tokio::test]
            #[ignore]
            async fn drops_columns_keys_and_checks() {
                super::drops_columns_keys_and_checks($engine).await;
            }

            #[tokio::test]
            #[ignore]
            async fn reads_and_drops_constraints_by_name() {
                super::reads_and_drops_constraints_by_name($engine).await;
            }

            #[tokio::test]
            #[ignore]
            async fn backup_keeps_keys_and_checks() {
                super::backup_keeps_keys_and_checks($engine).await;
            }

            #[tokio::test]
            #[ignore]
            async fn shows_create_statements() {
                super::shows_create_statements($engine).await;
            }

            #[tokio::test]
            #[ignore]
            async fn creates_and_drops_databases_and_schemas() {
                super::creates_and_drops_databases_and_schemas($engine).await;
            }

            #[tokio::test]
            #[ignore]
            async fn lists_and_defines_routines() {
                super::lists_and_defines_routines($engine).await;
            }

            #[tokio::test]
            #[ignore]
            async fn duplicates_structure_and_drops() {
                super::duplicates_structure_and_drops($engine).await;
            }
        }
    };
}

engine_tests!(mysql, Engine::MySql);
engine_tests!(postgres, Engine::Postgres);
engine_tests!(sqlserver, Engine::SqlServer);

#[tokio::test]
#[ignore]
async fn mysql_moves_a_column() {
    moves_a_column_on_mysql().await;
}

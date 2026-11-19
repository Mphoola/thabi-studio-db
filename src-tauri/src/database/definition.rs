//! An object's CREATE statement, for Show CREATE.
//!
//! The server's own words where it will give them (MySQL's SHOW CREATE,
//! PostgreSQL's pg_get_viewdef and pg_get_functiondef, SQL Server's
//! OBJECT_DEFINITION). A table on PostgreSQL or SQL Server has no such
//! statement, so it is rebuilt the way a backup rebuilds it, and says so.

use super::{Cell, Engine, ObjectKind, Session, dump, quote, values::Param};
use crate::errors::AppError;

fn first_text(result: &super::QueryResult, column: usize) -> Option<String> {
    match result.rows.first().and_then(|row| row.get(column)) {
        Some(Cell::Value(text)) => Some(text.clone()),
        _ => None,
    }
}

fn not_found(name: &str) -> AppError {
    AppError::new(
        "not_found",
        format!(
            "{name}'s definition couldn't be read. It may have been dropped, or this login can't see it."
        ),
    )
}

/// The CREATE statement for a table or view.
pub async fn object_definition(
    session: &mut Session,
    database: &str,
    schema: &str,
    name: &str,
    kind: ObjectKind,
) -> Result<String, AppError> {
    let engine = session.server_info().engine;
    let qualified = quote::table(engine, database, schema, name);
    if kind == ObjectKind::Table || kind == ObjectKind::ForeignTable {
        let described = session.describe(database, schema, name, kind).await?;
        let mut statements = dump::structure_statements(engine, database, schema, name, &described);
        // The backup's DROP line is for restoring, not for reading.
        statements.retain(|line| !line.starts_with("DROP TABLE"));
        let mut text = statements.concat();
        if engine != Engine::MySql {
            text = format!(
                "-- Rebuilt from the catalog: {} has no SHOW CREATE TABLE.\n{text}",
                engine.label()
            );
        }
        return Ok(text);
    }
    let text = match engine {
        Engine::MySql => {
            let result = session
                .execute(&format!("SHOW CREATE VIEW {qualified}"), 1)
                .await?;
            first_text(&result, 1).map(|sql| format!("{sql};"))
        }
        Engine::Postgres => {
            let result = session
                .execute_bound(
                    "SELECT pg_get_viewdef(c.oid, true) FROM pg_class c \
                     JOIN pg_namespace n ON n.oid = c.relnamespace \
                     WHERE n.nspname = $1 AND c.relname = $2",
                    &[Param::Text(schema.into()), Param::Text(name.into())],
                    1,
                )
                .await?;
            let keyword = if kind == ObjectKind::MaterializedView {
                "CREATE MATERIALIZED VIEW"
            } else {
                "CREATE OR REPLACE VIEW"
            };
            first_text(&result, 0)
                .map(|body| format!("{keyword} {qualified} AS\n{}", body.trim_end()))
        }
        // The module text, read from the object's own database: the session
        // may be in another one, and OBJECT_DEFINITION only sees its own.
        Engine::SqlServer => {
            let db = quote::ident(engine, database);
            let result = session
                .execute_bound(
                    &format!(
                        "SELECT m.definition FROM {db}.sys.sql_modules m \
                         JOIN {db}.sys.objects o ON o.object_id = m.object_id \
                         JOIN {db}.sys.schemas s ON s.schema_id = o.schema_id \
                         WHERE s.name = @P1 AND o.name = @P2"
                    ),
                    &[Param::Text(schema.into()), Param::Text(name.into())],
                    1,
                )
                .await?;
            first_text(&result, 0)
        }
    };
    text.ok_or_else(|| not_found(name))
}

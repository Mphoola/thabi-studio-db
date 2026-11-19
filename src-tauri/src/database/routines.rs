//! Functions, procedures, triggers, sequences and types: listed for the
//! explorer, and their definitions read for Show Definition.
//!
//! Each is identified by what its engine can look it up by without
//! ambiguity: an oid on PostgreSQL (functions can be overloaded, so a name
//! is not enough), an object id on SQL Server, the name on MySQL.

use super::{Cell, Engine, Session, quote, values::Param};
use crate::errors::AppError;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutineInfo {
    pub name: String,
    /// "function", "procedure", "trigger", "sequence" or "type".
    pub kind: String,
    /// What the definition is looked up by (oid, object id or name).
    pub identity: String,
    /// The table a trigger is on.
    pub table: Option<String>,
}

fn text(cell: Option<&Cell>) -> Option<String> {
    match cell {
        Some(Cell::Value(value)) => Some(value.clone()),
        _ => None,
    }
}

fn require_current(session: &Session, database: &str) -> Result<(), AppError> {
    let current = session.server_info().database.clone().unwrap_or_default();
    if current != database {
        return Err(super::error::database_not_open(database, &current));
    }
    Ok(())
}

/// Everything of these kinds in a schema (a database on MySQL).
pub async fn list_routines(
    session: &mut Session,
    database: &str,
    schema: &str,
) -> Result<Vec<RoutineInfo>, AppError> {
    let engine = session.server_info().engine;
    let (sql, params): (String, Vec<Param>) = match engine {
        Engine::Postgres => {
            require_current(session, database)?;
            (
                // Members of an extension (PostGIS's hundreds of functions)
                // are the extension's, not the schema's, and are left out.
                "WITH ext AS (SELECT objid FROM pg_depend WHERE deptype = 'e')
                 SELECT p.proname::text, CASE p.prokind WHEN 'p' THEN 'procedure' ELSE 'function' END,
                        p.oid::text, NULL
                 FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
                 WHERE n.nspname = $1 AND p.prokind IN ('f', 'p')
                   AND p.oid NOT IN (SELECT objid FROM ext)
                 UNION ALL
                 SELECT t.tgname::text, 'trigger', t.oid::text, c.relname::text
                 FROM pg_trigger t JOIN pg_class c ON c.oid = t.tgrelid
                 JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = $1 AND NOT t.tgisinternal
                 UNION ALL
                 SELECT c.relname::text, 'sequence', c.oid::text, NULL
                 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
                 WHERE n.nspname = $1 AND c.relkind = 'S' AND c.oid NOT IN (SELECT objid FROM ext)
                 UNION ALL
                 SELECT t.typname::text, 'type', t.oid::text, NULL
                 FROM pg_type t JOIN pg_namespace n ON n.oid = t.typnamespace
                 WHERE n.nspname = $1 AND t.typtype IN ('e', 'd')
                   AND t.oid NOT IN (SELECT objid FROM ext)
                 ORDER BY 2, 1"
                    .to_owned(),
                vec![Param::Text(schema.into())],
            )
        }
        Engine::MySql => (
            "SELECT CONVERT(ROUTINE_NAME USING utf8mb4), LOWER(ROUTINE_TYPE),
                    CONVERT(ROUTINE_NAME USING utf8mb4), NULL
             FROM information_schema.ROUTINES WHERE ROUTINE_SCHEMA = ?
             UNION ALL
             SELECT CONVERT(TRIGGER_NAME USING utf8mb4), 'trigger',
                    CONVERT(TRIGGER_NAME USING utf8mb4), CONVERT(EVENT_OBJECT_TABLE USING utf8mb4)
             FROM information_schema.TRIGGERS WHERE TRIGGER_SCHEMA = ?
             ORDER BY 2, 1"
                .to_owned(),
            vec![Param::Text(database.into()), Param::Text(database.into())],
        ),
        Engine::SqlServer => {
            let db = quote::ident(engine, database);
            (
                format!(
                    "SELECT o.name,
                            CASE o.type WHEN 'P' THEN 'procedure' WHEN 'TR' THEN 'trigger'
                                        WHEN 'SO' THEN 'sequence' ELSE 'function' END,
                            CAST(o.object_id AS nvarchar(20)), p.name
                     FROM {db}.sys.objects o
                     JOIN {db}.sys.schemas s ON s.schema_id = o.schema_id
                     LEFT JOIN {db}.sys.objects p ON p.object_id = o.parent_object_id
                     WHERE s.name = @P1 AND o.is_ms_shipped = 0
                       AND o.type IN ('P', 'FN', 'IF', 'TF', 'TR', 'SO')
                     UNION ALL
                     SELECT t.name, 'type', CAST(t.user_type_id AS nvarchar(20)), NULL
                     FROM {db}.sys.types t JOIN {db}.sys.schemas s ON s.schema_id = t.schema_id
                     WHERE t.is_user_defined = 1 AND s.name = @P1
                     ORDER BY 2, 1"
                ),
                vec![Param::Text(schema.into())],
            )
        }
    };
    let result = session.execute_bound(&sql, &params, 100_000).await?;
    Ok(result
        .rows
        .iter()
        .filter_map(|row| {
            Some(RoutineInfo {
                name: text(row.first())?,
                kind: text(row.get(1))?,
                identity: text(row.get(2))?,
                table: text(row.get(3)),
            })
        })
        .collect())
}

fn not_readable(kind: &str) -> AppError {
    AppError::new(
        "not_found",
        format!(
            "The {kind}'s definition couldn't be read. It may have been dropped, or this login can't see it."
        ),
    )
}

/// The CREATE statement for one of them, as the server states it where it
/// will, and built from the catalog where it won't (sequences and types).
pub async fn routine_definition(
    session: &mut Session,
    database: &str,
    kind: &str,
    identity: &str,
) -> Result<String, AppError> {
    let engine = session.server_info().engine;
    let id = Param::Text(identity.into());
    let (sql, params, column): (String, Vec<Param>, usize) = match (engine, kind) {
        (Engine::Postgres, "function" | "procedure") => (
            "SELECT pg_get_functiondef($1::text::oid)".into(),
            vec![id],
            0,
        ),
        (Engine::Postgres, "trigger") => (
            "SELECT pg_get_triggerdef($1::text::oid, true) || ';'".into(),
            vec![id],
            0,
        ),
        (Engine::Postgres, "sequence") => (
            "SELECT format(E'CREATE SEQUENCE %s\\n  AS %s\\n  INCREMENT BY %s\\n  MINVALUE %s\\n  MAXVALUE %s\\n  START WITH %s\\n  CACHE %s%s;',
                    s.seqrelid::regclass, format_type(s.seqtypid, NULL), s.seqincrement,
                    s.seqmin, s.seqmax, s.seqstart, s.seqcache,
                    CASE WHEN s.seqcycle THEN E'\\n  CYCLE' ELSE '' END)
             FROM pg_sequence s WHERE s.seqrelid = $1::text::oid"
                .into(),
            vec![id],
            0,
        ),
        (Engine::Postgres, "type") => (
            "SELECT CASE t.typtype
                      WHEN 'e' THEN format('CREATE TYPE %s AS ENUM (%s);', t.oid::regtype,
                        (SELECT string_agg(quote_literal(e.enumlabel), ', ' ORDER BY e.enumsortorder)
                         FROM pg_enum e WHERE e.enumtypid = t.oid))
                      ELSE format('CREATE DOMAIN %s AS %s%s%s%s;', t.oid::regtype,
                        format_type(t.typbasetype, t.typtypmod),
                        CASE WHEN t.typnotnull THEN ' NOT NULL' ELSE '' END,
                        coalesce(' DEFAULT ' || t.typdefault, ''),
                        coalesce((SELECT string_agg(' CONSTRAINT ' || quote_ident(c.conname) || ' '
                                                    || pg_get_constraintdef(c.oid), '')
                                  FROM pg_constraint c WHERE c.contypid = t.oid), ''))
                    END
             FROM pg_type t WHERE t.oid = $1::text::oid"
                .into(),
            vec![id],
            0,
        ),
        // SHOW CREATE puts the statement in the third column for all three.
        (Engine::MySql, "function" | "procedure" | "trigger") => (
            format!(
                "SHOW CREATE {} {}",
                kind.to_uppercase(),
                quote::table(engine, database, "", identity)
            ),
            vec![],
            2,
        ),
        (Engine::SqlServer, "function" | "procedure" | "trigger") => {
            let db = quote::ident(engine, database);
            (
                format!(
                    "SELECT m.definition FROM {db}.sys.sql_modules m \
                     WHERE m.object_id = CAST(@P1 AS int)"
                ),
                vec![id],
                0,
            )
        }
        (Engine::SqlServer, "sequence") => {
            let db = quote::ident(engine, database);
            (
                format!(
                    "SELECT CONCAT('CREATE SEQUENCE ', QUOTENAME(s.name), '.', QUOTENAME(q.name),
                                   CHAR(10), '  AS ', TYPE_NAME(q.user_type_id),
                                   CHAR(10), '  START WITH ', CAST(q.start_value AS nvarchar(40)),
                                   CHAR(10), '  INCREMENT BY ', CAST(q.increment AS nvarchar(40)),
                                   CHAR(10), '  MINVALUE ', CAST(q.minimum_value AS nvarchar(40)),
                                   CHAR(10), '  MAXVALUE ', CAST(q.maximum_value AS nvarchar(40)),
                                   CASE WHEN q.is_cycling = 1 THEN CONCAT(CHAR(10), '  CYCLE') ELSE '' END, ';')
                     FROM {db}.sys.sequences q JOIN {db}.sys.schemas s ON s.schema_id = q.schema_id
                     WHERE q.object_id = CAST(@P1 AS int)"
                ),
                vec![id],
                0,
            )
        }
        (Engine::SqlServer, "type") => {
            let db = quote::ident(engine, database);
            (
                format!(
                    "SELECT CASE WHEN t.is_table_type = 1
                              THEN CONCAT('-- ', QUOTENAME(s.name), '.', QUOTENAME(t.name),
                                          ' is a table type; SQL Server does not state its definition.')
                              ELSE CONCAT('CREATE TYPE ', QUOTENAME(s.name), '.', QUOTENAME(t.name),
                                   ' FROM ', TYPE_NAME(t.system_type_id),
                                   CASE WHEN TYPE_NAME(t.system_type_id) IN ('varchar', 'char', 'varbinary', 'binary')
                                        THEN CONCAT('(', IIF(t.max_length = -1, 'MAX', CAST(t.max_length AS nvarchar(10))), ')')
                                        WHEN TYPE_NAME(t.system_type_id) IN ('nvarchar', 'nchar')
                                        THEN CONCAT('(', IIF(t.max_length = -1, 'MAX', CAST(t.max_length / 2 AS nvarchar(10))), ')')
                                        WHEN TYPE_NAME(t.system_type_id) IN ('decimal', 'numeric')
                                        THEN CONCAT('(', t.precision, ', ', t.scale, ')')
                                        ELSE '' END,
                                   IIF(t.is_nullable = 1, ' NULL', ' NOT NULL'), ';')
                            END
                     FROM {db}.sys.types t JOIN {db}.sys.schemas s ON s.schema_id = t.schema_id
                     WHERE t.user_type_id = CAST(@P1 AS int)"
                ),
                vec![id],
                0,
            )
        }
        _ => return Err(not_readable(kind)),
    };
    let result = if params.is_empty() {
        session.execute(&sql, 1).await?
    } else {
        session.execute_bound(&sql, &params, 1).await?
    };
    result
        .rows
        .first()
        .and_then(|row| text(row.get(column)))
        .ok_or_else(|| not_readable(kind))
}

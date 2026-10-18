//! Writing a schema and its rows out as a `.sql` file.
//!
//! Thabi writes the file itself rather than running `pg_dump` or `mysqldump`.
//! That costs exactness on two engines and buys working everywhere: the native
//! tools are not installed on most machines, `sqlpackage` almost never is, and
//! an app whose backup depends on a binary it cannot install is not a backup.
//!
//! MySQL is the exception: it has `SHOW CREATE TABLE`, which is the server's
//! own words rather than our reconstruction of them, so it is used. The other
//! two are rebuilt from `describe`, which is faithful for a table Thabi can
//! describe and silent about anything it cannot — see `LIMITATIONS`.
//!
//! Rows are streamed a page at a time and written as they arrive, so a table
//! with a hundred million rows costs a page of memory rather than a file's.

use std::io::Write;

use super::metadata::ObjectStructure;
use super::transfer;
use super::values::Cell;
use super::{Engine, quote};
use crate::errors::AppError;

/// Rows per INSERT statement.
///
/// Multi-row INSERTs are an order of magnitude fewer round trips, but a
/// statement with a few thousand values can exceed `max_allowed_packet` and be
/// refused outright, so the batch is counted in values rather than rows and
/// kept small enough for the smallest default packet size.
pub const VALUES_PER_STATEMENT: usize = 500;

/// What to write.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DumpOptions {
    pub structure: bool,
    pub data: bool,
    /// Empty means every table in the schema.
    #[serde(default)]
    pub tables: Vec<String>,
    /// Row cap, for a sample rather than a whole table. None exports all.
    #[serde(default)]
    pub row_limit: Option<u64>,
}

impl DumpOptions {
    /// Neither chosen would produce a file with nothing in it.
    pub fn wants_anything(&self) -> bool {
        self.structure || self.data
    }
}

/// What happened, for the summary the user is shown.
#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DumpSummary {
    pub tables: usize,
    pub rows: u64,
    pub bytes: u64,
}

/// A comment at the top of the file.
///
/// Says what produced it and when, because a `.sql` file found in a folder six
/// months later with no idea which server it came from is the file version of
/// an unlabelled backup drive.
pub fn header(engine: Engine, database: &str, schema: &str, generated: &str) -> String {
    let version = env!("CARGO_PKG_VERSION");
    format!(
        "-- Thabi Studio {version} export\n\
         -- Engine: {}\n\
         -- Database: {database}\n\
         -- Schema: {}\n\
         -- Generated: {generated}\n",
        engine.label(),
        if schema.is_empty() { "-" } else { schema },
    )
}

/// What the header says is inside, from the options that were asked for.
///
/// Written rather than implied, because a backup that is missing its data
/// should say so at the top rather than at the point somebody tries to
/// restore it.
pub fn contents_note(options: &DumpOptions) -> String {
    match (options.structure, options.data) {
        (true, true) => "-- Contains table structure and data.\n".to_owned(),
        (true, false) => "-- Contains table structure only.\n".to_owned(),
        (false, true) => "-- Contains data only.\n".to_owned(),
        (false, false) => "-- Nothing was selected.\n".to_owned(),
    }
}

/// The warning for an engine whose structure is rebuilt rather than quoted.
///
/// PostgreSQL and SQL Server have no `SHOW CREATE TABLE`, so their CREATE
/// TABLE statements in this file are rebuilt from the catalog. That is faithful
/// for columns, types, nullability, defaults, keys and indexes, and blind to
/// check constraints, triggers and partitions. A reader needs to know which
/// kind of file they are holding before they trust it as a backup.
pub fn fidelity_note(engine: Engine, options: &DumpOptions) -> Option<String> {
    if !options.structure || engine == Engine::MySql {
        return None;
    }
    Some(format!(
        "--\n\
         -- {} has no SHOW CREATE TABLE, so CREATE TABLE statements below were\n\
         -- rebuilt from the catalog. They carry columns, types, nullability,\n\
         -- defaults, primary keys, unique constraints, indexes and\n\
         -- single-column foreign keys — not check constraints, triggers or\n\
         -- partitions. mysqldump or pg_dump would be needed for those.\n",
        engine.label()
    ))
}

/// `INSERT INTO t (a, b) VALUES (...), (...)` for as many rows as fit.
///
/// Written as one function so the value escaping lives in exactly one place:
/// `transfer::literal` already knows how each engine spells NULL, a quoted
/// string and a blob, and a second opinion here would be one more place to be
/// wrong.
pub fn insert_statement(
    engine: Engine,
    table: &str,
    columns: &[String],
    rows: &[Vec<Cell>],
) -> Result<String, AppError> {
    if rows.is_empty() || columns.is_empty() {
        return Ok(String::new());
    }
    let names = columns
        .iter()
        .map(|name| quote::ident(engine, name))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = format!("INSERT INTO {table} ({names}) VALUES ");
    let mut tuples = Vec::with_capacity(rows.len());
    for row in rows {
        // A row whose shape does not match the header would produce a
        // statement that fails at the far end of a long import, so it is
        // caught here where the file is being written.
        if row.len() != columns.len() {
            return Err(AppError::new(
                "row_shape_mismatch",
                "A row did not have the same number of values as the table has columns.",
            ));
        }
        let mut cells = Vec::with_capacity(row.len());
        for cell in row {
            cells.push(transfer::literal(cell, engine));
        }
        tuples.push(format!("({})", cells.join(", ")));
    }
    out.push_str(&tuples.join(", "));
    out.push_str(";\n");
    Ok(out)
}

/// A `TableSpec` from what `describe` reported, for a dump.
///
/// Distinct from `ddl::spec_from_structure`, which builds the spec for
/// *duplicating* a table and deliberately takes only what CREATE TABLE needs.
/// A dump that dropped unique constraints and foreign keys would not restore
/// the schema, so this carries them from the structure as well.
fn spec_for_dump(name: &str, structure: &ObjectStructure) -> super::ddl::TableSpec {
    use super::ddl::{ColumnSpec, ForeignKey, TableSpec};

    let indexes = structure.indexes.as_deref().unwrap_or(&[]);
    // The primary key goes inside CREATE TABLE, so it must not also be a
    // unique constraint or the table would get two differently named keys.
    let uniques: Vec<&str> = indexes
        .iter()
        .filter(|index| !index.primary && index.unique)
        .flat_map(|index| index.columns.iter().map(String::as_str))
        .collect();

    let columns = structure
        .columns
        .iter()
        .map(|column| ColumnSpec {
            name: column.name.clone(),
            data_type: column.data_type.clone(),
            nullable: column.nullable,
            default: column.default.clone().unwrap_or_default(),
            comment: String::new(),
            primary_key: column.primary_key.is_some(),
            unique: uniques.contains(&column.name.as_str()),
        })
        .collect();

    let foreign_keys = structure
        .columns
        .iter()
        .filter_map(|column| {
            let reference = column.references.as_ref()?;
            Some(ForeignKey {
                name: format!("{}_{}_fk", name, column.name),
                columns: vec![column.name.clone()],
                references: {
                    let mut parts = vec![reference.schema.clone()];
                    parts.push(reference.table.clone());
                    parts.retain(|part| !part.is_empty());
                    parts
                },
                referenced_columns: vec![reference.column.clone()],
                on_delete: "NO ACTION".to_owned(),
                on_update: "NO ACTION".to_owned(),
            })
        })
        .collect();

    TableSpec {
        columns,
        unique_names: Vec::new(),
        primary_key_name: String::new(),
        foreign_keys,
    }
}

/// A table's name as SQL, so a caller can address the same table the file was
/// written for without re-deriving the quoting and getting it differently.
pub fn table_of(engine: Engine, database: &str, schema: &str, name: &str) -> String {
    quote::table(engine, database, schema, name)
}

/// A statement ending in exactly one semicolon.
///
/// The generators do not agree: `create_table` leaves one off and
/// `create_index` leaves one on, so a file had `);;` in some places and
/// nothing at all in others. A file that is read by people as well as by
/// servers should not have that in it either way.
fn one_statement(sql: &str) -> String {
    format!("{};", sql.trim_end().trim_end_matches(';'))
}

/// A `DROP TABLE IF EXISTS` before the create, so a restore into a populated
/// database replaces rather than collides.
pub fn drop_table(table: &str) -> String {
    format!("DROP TABLE IF EXISTS {table};\n")
}

/// Emits one table's structure.
///
/// MySQL is asked; the others are rebuilt from what `describe` reported. A
/// rebuild is faithful for columns, types, nullability, defaults, enum values,
/// primary keys, indexes and single-column foreign keys. Anything the
/// explorer cannot see — a check constraint, a trigger, a partition, a comment
/// — is not in the file, and the file says so rather than looking complete.
pub fn structure_statements(
    engine: Engine,
    database: &str,
    schema: &str,
    name: &str,
    described: &ObjectStructure,
) -> Vec<String> {
    let table = quote::table(engine, database, schema, name);
    let mut out = vec![drop_table(&table)];

    if let Some(shown) = described.show_create.clone() {
        // The server's own words, which beat anything reconstructed.
        out.push(format!("{};\n", shown.trim_end().trim_end_matches(';')));
        return out;
    }

    let spec = spec_for_dump(name, described);
    match super::ddl::create_table(engine, database, schema, name, &spec) {
        Ok(created) => {
            out.push(format!("{}\n", one_statement(&created)));
            for index in described.indexes.iter().flatten() {
                // The primary key is written inside CREATE TABLE, so writing it
                // again would be a second, differently named constraint.
                if index.primary {
                    continue;
                }
                let spec = super::ddl::IndexSpec {
                    name: index.name.clone(),
                    columns: index.columns.clone(),
                    unique: index.unique,
                };
                out.push(format!(
                    "{}\n",
                    one_statement(&super::ddl::create_index(
                        engine, database, schema, name, &spec
                    ))
                ));
            }
        }
        Err(reason) => {
            // A table we cannot describe cannot be recreated, and a silent gap
            // in a backup is the worst kind.
            out.push(format!(
                "-- Could not write the structure of {table}: {reason}\n"
            ));
        }
    }
    out
}

fn write_failed(cause: std::io::Error) -> AppError {
    AppError::new("write_failed", "Couldn't write the file.").with_detail(Some(cause.to_string()))
}

/// Counts what was written, without buffering it.
pub struct CountingWriter<W: Write> {
    inner: W,
    bytes: u64,
}

impl<W: Write> CountingWriter<W> {
    pub fn new(inner: W) -> Self {
        Self { inner, bytes: 0 }
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.bytes += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// The tables a dump will write, and the structures to write them from.
///
/// Decided before anything is written, so a schema with no readable table
/// produces an error naming it rather than a file that quietly restores
/// nothing.
pub async fn plan(
    session: &mut super::Session,
    database: &str,
    schema: &str,
    options: &DumpOptions,
) -> Result<Vec<(String, ObjectStructure)>, AppError> {
    let engine = session.server_info().engine;
    let listed = session
        .list_objects(database, schema)
        .await
        .unwrap_or_default();
    let wanted: Vec<String> = listed
        .iter()
        // Views are not tables: DROP TABLE IF EXISTS and CREATE VIEW together
        // make a file that cannot run.
        .filter(|object| object.kind == super::ObjectKind::Table)
        .map(|object| object.name.clone())
        .filter(|name| options.tables.is_empty() || options.tables.contains(name))
        .collect();
    if wanted.is_empty() {
        return Err(AppError::new(
            "nothing_to_export",
            "There are no tables here to export.",
        ));
    }

    let mut read = Vec::new();
    for name in wanted {
        match session
            .describe(database, schema, &name, super::ObjectKind::Table)
            .await
        {
            Ok(structure) => read.push((name, structure)),
            Err(reason) => {
                // One unreadable table is not a reason to write a backup that
                // is missing it without saying so.
                return Err(AppError::new(
                    "table_unreadable",
                    format!("Couldn't read the structure of {name}."),
                )
                .with_detail(Some(reason.message)));
            }
        }
    }
    let _ = engine;
    Ok(order_for_restore(read))
}

/// Tables in an order a restore can actually run.
///
/// Alphabetical order puts `orders` before the `customers` it references, and
/// the file then fails on its first foreign key. Referenced tables come first.
/// Two tables that reference each other have no such order — the database
/// allowed it, so the dump cannot invent one — and those keep the order they
/// were listed in rather than looping forever.
fn order_for_restore(tables: Vec<(String, ObjectStructure)>) -> Vec<(String, ObjectStructure)> {
    let names: Vec<String> = tables.iter().map(|(name, _)| name.clone()).collect();
    let needs: Vec<Vec<usize>> = tables
        .iter()
        .map(|(_, structure)| {
            structure
                .columns
                .iter()
                .filter_map(|column| column.references.as_ref())
                .filter_map(|reference| names.iter().position(|n| *n == reference.table))
                .collect()
        })
        .collect();

    let mut done: Vec<usize> = Vec::new();
    let mut order: Vec<usize> = Vec::new();
    while order.len() < tables.len() {
        let before = order.len();
        for (index, required) in needs.iter().enumerate() {
            if done.contains(&index) {
                continue;
            }
            if required.iter().all(|target| done.contains(target)) {
                order.push(index);
                done.push(index);
            }
        }
        if order.len() == before {
            // A cycle: nothing became placeable this sweep.
            order.extend((0..tables.len()).filter(|index| !done.contains(index)));
            break;
        }
    }
    order
        .into_iter()
        .map(|index| tables[index].clone())
        .collect()
}

/// Writes one table's rows as INSERT statements, streaming them.
///
/// Rows arrive a page at a time and are written as they come, so memory is one
/// batch rather than one table. Paging is ordered by the table's key, without
/// which a page boundary can repeat or skip rows — and a backup with a repeated
/// row is worse than one that failed outright.
///
/// Takes the connection's session rather than a query tab's, so a schema can
/// be backed up whether or not anything is open, and so the round-trip test
/// can drive it directly.
///
/// The batch is counted in values rather than rows: a multi-row INSERT with a
/// few thousand of them is refused with "packets too large", and a backup that
/// fails on the largest table in the database is not a backup.
pub async fn write_table_rows(
    session: &mut super::Session,
    database: &str,
    schema: &str,
    name: &str,
    structure: &ObjectStructure,
    limit: Option<u64>,
    out: &mut impl Write,
) -> Result<u64, AppError> {
    use super::paging::page_sql;

    let engine = session.server_info().engine;
    let columns: Vec<String> = structure
        .columns
        .iter()
        .map(|column| column.name.clone())
        .collect();
    if columns.is_empty() {
        return Ok(0);
    }
    let primary_key: Vec<String> = structure
        .columns
        .iter()
        .filter_map(|column| column.primary_key.map(|_| column.name.clone()))
        .collect();

    let table = quote::table(engine, database, schema, name);
    let mut batch: Vec<Vec<Cell>> = Vec::new();
    let mut batch_values = 0usize;
    let mut written = 0u64;
    let mut offset = 0usize;

    // Every column, in order: a dump that dropped columns would restore a
    // table that no longer matches the one it came from.
    let all: Vec<usize> = (0..columns.len()).collect();
    let total = limit.map(|cap| cap as usize).unwrap_or(usize::MAX);

    while offset < total {
        let want = PAGE.min(total - offset);
        let sql = page_sql(
            engine,
            database,
            schema,
            name,
            "",
            None,
            &primary_key,
            offset,
            want,
        );
        let result = session.execute(&sql, want).await?;
        if result.rows.is_empty() {
            break;
        }
        for row in &result.rows {
            let values: Vec<Cell> = all.iter().map(|index| row[*index].clone()).collect();
            batch_values += values.len();
            batch.push(values);
            if batch_values >= VALUES_PER_STATEMENT {
                let statement = insert_statement(engine, &table, &columns, &batch)?;
                out.write_all(statement.as_bytes()).map_err(write_failed)?;
                written += batch.len() as u64;
                batch.clear();
                batch_values = 0;
            }
        }
        let fetched = result.rows.len();
        offset += fetched;
        if fetched < want {
            break;
        }
    }

    if !batch.is_empty() {
        let statement = insert_statement(engine, &table, &columns, &batch)?;
        out.write_all(statement.as_bytes()).map_err(write_failed)?;
        written += batch.len() as u64;
    }
    Ok(written)
}

/// Rows read per page. Small on purpose: a page is held in memory while it is
/// turned into a statement, and the batch that becomes a file is on top.
const PAGE: usize = 500;

#[cfg(test)]
#[path = "dump/tests.rs"]
mod tests;

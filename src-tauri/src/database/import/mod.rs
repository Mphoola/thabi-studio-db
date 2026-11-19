//! Importing a CSV, TSV or JSON file into a table (Phase 15).
//!
//! Rows are read one at a time and inserted in multi-row batches with bound
//! values, inside one transaction, so a file of any size imports with flat
//! memory and a failure leaves the table as it was. Each batch has a savepoint,
//! so a row the server refuses can be found and named by its line: then the
//! import either stops and rolls everything back, or skips that row and goes
//! on, as the person chose. Never retried.

pub mod reader;

pub use reader::{Encoding, Format, Preview, ReadOptions, preview};

use super::{Engine, ObjectKind, Session, quote, values::Param};
use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTarget {
    pub database: String,
    #[serde(default)]
    pub schema: Option<String>,
    pub table: String,
}

/// Which file column fills which table column.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMapping {
    pub file_column: usize,
    pub column: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOptions {
    #[serde(flatten)]
    pub read: ReadOptions,
    /// An empty field becomes NULL rather than an empty string.
    #[serde(default = "yes")]
    pub empty_as_null: bool,
    /// Skip a row the server refuses and go on, instead of stopping.
    #[serde(default)]
    pub skip_bad_rows: bool,
}

fn yes() -> bool {
    true
}

/// Shared with the progress command while an import runs.
#[derive(Debug, Default)]
pub struct ImportProgress {
    pub rows: AtomicU64,
    pub rejected: AtomicU64,
    pub bytes: AtomicU64,
    pub total_bytes: AtomicU64,
    pub cancel: AtomicBool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rejected {
    pub line: u64,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub inserted: u64,
    /// The first rejected rows, with their lines (at most 1,000 kept).
    pub rejected: Vec<Rejected>,
    pub rejected_total: u64,
    pub cancelled: bool,
    /// Nothing was kept: stopped on a bad row, or cancelled.
    pub rolled_back: bool,
}

const MAX_REJECTED_KEPT: usize = 1000;

struct Sql {
    begin: &'static str,
    savepoint: &'static str,
    back_to: &'static str,
    release: Option<&'static str>,
}

fn sql_for(engine: Engine) -> Sql {
    match engine {
        Engine::Postgres => Sql {
            begin: "BEGIN",
            savepoint: "SAVEPOINT thabi_import",
            back_to: "ROLLBACK TO SAVEPOINT thabi_import",
            release: Some("RELEASE SAVEPOINT thabi_import"),
        },
        Engine::MySql => Sql {
            begin: "START TRANSACTION",
            savepoint: "SAVEPOINT thabi_import",
            back_to: "ROLLBACK TO SAVEPOINT thabi_import",
            release: Some("RELEASE SAVEPOINT thabi_import"),
        },
        Engine::SqlServer => Sql {
            begin: "BEGIN TRANSACTION",
            savepoint: "SAVE TRANSACTION thabi_import",
            back_to: "ROLLBACK TRANSACTION thabi_import",
            release: None,
        },
    }
}

/// Values per statement, under each engine's limit on bound parameters.
fn max_params(engine: Engine) -> usize {
    match engine {
        Engine::SqlServer => 2000,
        _ => 30_000,
    }
}

/// `INSERT INTO t (a, b) VALUES (…), (…)` for `rows` rows, with placeholders.
/// PostgreSQL casts each to the column's type as the server named it, since
/// a text value is not assigned to an integer column without one.
pub fn insert_sql(
    engine: Engine,
    table: &str,
    columns: &[(String, String)],
    rows: usize,
) -> String {
    let names = columns
        .iter()
        .map(|(name, _)| quote::ident(engine, name))
        .collect::<Vec<_>>()
        .join(", ");
    let mut n = 0;
    let groups = (0..rows)
        .map(|_| {
            let values = columns
                .iter()
                .map(|(_, data_type)| {
                    n += 1;
                    match engine {
                        Engine::Postgres => format!("CAST(${n} AS {data_type})"),
                        Engine::MySql => "?".to_owned(),
                        Engine::SqlServer => format!("@P{n}"),
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("({values})")
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("INSERT INTO {table} ({names}) VALUES {groups}")
}

struct Row {
    line: u64,
    values: Vec<Param>,
}

/// Imports the file. The session should be a tab's own, not the connection's
/// main one: this holds it for as long as the file takes.
pub async fn run(
    session: &mut Session,
    target: &ImportTarget,
    mappings: &[ColumnMapping],
    options: &ImportOptions,
    path: &str,
    progress: &ImportProgress,
) -> Result<ImportSummary, AppError> {
    let engine = session.server_info().engine;
    let schema = target.schema.clone().unwrap_or_default();
    // The table's columns as the server describes them: the names must exist,
    // and the types used in casts are the server's words, not the webview's.
    let structure = session
        .describe(&target.database, &schema, &target.table, ObjectKind::Table)
        .await?;
    let types: HashMap<&str, &str> = structure
        .columns
        .iter()
        .map(|c| (c.name.as_str(), c.data_type.as_str()))
        .collect();
    if mappings.is_empty() {
        return Err(AppError::new(
            "validation",
            "Choose at least one column to import.",
        ));
    }
    let columns: Vec<(String, String)> = mappings
        .iter()
        .map(|m| {
            types
                .get(m.column.as_str())
                .map(|t| (m.column.clone(), (*t).to_owned()))
                .ok_or_else(|| {
                    AppError::new(
                        "validation",
                        format!("{} has no column {}.", target.table, m.column),
                    )
                })
        })
        .collect::<Result<_, _>>()?;
    let table = quote::table(engine, &target.database, &schema, &target.table);
    let per_batch = (max_params(engine) / columns.len()).clamp(1, 500);

    let mut reader = reader::Reader::open(path, &options.read)?;
    progress
        .total_bytes
        .store(reader.total_bytes(), Ordering::Relaxed);
    let sql = sql_for(engine);
    let mut summary = ImportSummary::default();
    session.execute(sql.begin, 0).await?;

    let outcome: Result<(), AppError> = async {
        loop {
            if progress.cancel.load(Ordering::SeqCst) {
                summary.cancelled = true;
                return Ok(());
            }
            let mut batch = Vec::with_capacity(per_batch);
            while batch.len() < per_batch {
                let Some(record) = reader.next_record()? else {
                    break;
                };
                batch.push(Row {
                    line: record.line,
                    values: mappings
                        .iter()
                        .map(|m| {
                            value(
                                engine,
                                record.fields.get(m.file_column).cloned().flatten(),
                                options,
                            )
                        })
                        .collect(),
                });
            }
            if batch.is_empty() {
                return Ok(());
            }
            if engine == Engine::SqlServer {
                let bad = unconvertible(session, &columns, &batch).await?;
                if !bad.is_empty() {
                    for (index, message) in &bad {
                        summary.rejected_total += 1;
                        progress.rejected.fetch_add(1, Ordering::Relaxed);
                        if summary.rejected.len() < MAX_REJECTED_KEPT {
                            summary.rejected.push(Rejected {
                                line: batch[*index].line,
                                message: message.clone(),
                            });
                        }
                    }
                    if !options.skip_bad_rows {
                        return Ok(());
                    }
                    let skip: Vec<usize> = bad.iter().map(|(i, _)| *i).collect();
                    batch = batch
                        .into_iter()
                        .enumerate()
                        .filter(|(i, _)| !skip.contains(i))
                        .map(|(_, row)| row)
                        .collect();
                    if batch.is_empty() {
                        continue;
                    }
                }
            }
            session.execute(sql.savepoint, 0).await?;
            let params: Vec<Param> = batch
                .iter()
                .flat_map(|r| r.values.iter().cloned())
                .collect();
            match attempt(
                session,
                engine,
                &insert_sql(engine, &table, &columns, batch.len()),
                &params,
            )
            .await?
            {
                None => {
                    if let Some(release) = sql.release {
                        session.execute(release, 0).await?;
                    }
                    summary.inserted += batch.len() as u64;
                }
                Some(_) => {
                    // Find the row: back to before the batch, then one at a time.
                    session.execute(sql.back_to, 0).await?;
                    for row in &batch {
                        session.execute(sql.savepoint, 0).await?;
                        match attempt(
                            session,
                            engine,
                            &insert_sql(engine, &table, &columns, 1),
                            &row.values,
                        )
                        .await?
                        {
                            None => {
                                if let Some(release) = sql.release {
                                    session.execute(release, 0).await?;
                                }
                                summary.inserted += 1;
                            }
                            Some(message) => {
                                session.execute(sql.back_to, 0).await?;
                                summary.rejected_total += 1;
                                progress.rejected.fetch_add(1, Ordering::Relaxed);
                                if summary.rejected.len() < MAX_REJECTED_KEPT {
                                    summary.rejected.push(Rejected {
                                        line: row.line,
                                        message,
                                    });
                                }
                                if !options.skip_bad_rows {
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
            progress.rows.store(summary.inserted, Ordering::Relaxed);
            progress.bytes.store(reader.position(), Ordering::Relaxed);
        }
    }
    .await;

    let stop = outcome.is_err()
        || summary.cancelled
        || (summary.rejected_total > 0 && !options.skip_bad_rows);
    if stop {
        // Nothing is kept: a half-imported file is worse than none.
        let _ = session.execute("ROLLBACK", 0).await;
        summary.rolled_back = true;
        summary.inserted = 0;
        outcome?;
    } else {
        session.execute("COMMIT", 0).await?;
    }
    progress.rows.store(summary.inserted, Ordering::Relaxed);
    Ok(summary)
}

/// Runs an insert. `None` when it went in, the server's reason when it was
/// refused, and an error when the session or the transaction is gone.
async fn attempt(
    session: &mut Session,
    engine: Engine,
    sql: &str,
    params: &[Param],
) -> Result<Option<String>, AppError> {
    match session.execute_bound(sql, params, 0).await {
        Ok(_) => Ok(None),
        Err(error) if super::error::is_connection_failure(&error) => Err(error),
        Err(error) => {
            let reason = error
                .detail
                .clone()
                .unwrap_or_else(|| error.message.clone());
            // SQL Server dooms the transaction on some errors; then nothing
            // can be kept, and saying so beats a confusing second error.
            if engine == Engine::SqlServer {
                let state = session.execute("SELECT XACT_STATE()", 1).await?;
                if matches!(state.rows.first().and_then(|r| r.first()), Some(super::Cell::Value(v)) if v == "-1")
                {
                    return Err(AppError::new(
                        "import_aborted",
                        "SQL Server ended the import's transaction, so nothing was kept.",
                    )
                    .with_detail(Some(reason)));
                }
            }
            Ok(Some(reason))
        }
    }
}

/// SQL Server: the rows in a batch whose values won't convert to their
/// columns' types, found with TRY_CAST before inserting. Such a value dooms
/// the transaction on SQL Server, where the other engines just refuse the row.
async fn unconvertible(
    session: &mut Session,
    columns: &[(String, String)],
    batch: &[Row],
) -> Result<Vec<(usize, String)>, AppError> {
    // Types TRY_CAST can't take from text, or that any text fits.
    let checked: Vec<usize> = columns
        .iter()
        .enumerate()
        .filter(|(_, (_, data_type))| {
            let base = data_type
                .split('(')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            !matches!(
                base.as_str(),
                "char"
                    | "varchar"
                    | "nchar"
                    | "nvarchar"
                    | "text"
                    | "ntext"
                    | "xml"
                    | "sql_variant"
                    | "binary"
                    | "varbinary"
                    | "image"
                    | "geography"
                    | "geometry"
                    | "hierarchyid"
                    | "sysname"
                    | ""
            )
        })
        .map(|(i, _)| i)
        .collect();
    if checked.is_empty() {
        return Ok(Vec::new());
    }
    let mut params = Vec::new();
    let mut n = 0;
    let rows = batch
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let cells = checked
                .iter()
                .map(|&c| {
                    params.push(row.values[c].clone());
                    n += 1;
                    format!("@P{n}")
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("({i}, {cells})")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let names = (0..checked.len())
        .map(|k| format!("c{k}"))
        .collect::<Vec<_>>()
        .join(", ");
    let tests = checked
        .iter()
        .enumerate()
        .map(|(k, &c)| {
            format!(
                "WHEN v.c{k} IS NOT NULL AND TRY_CAST(v.c{k} AS {}) IS NULL THEN {k}",
                columns[c].1
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let sql = format!(
        "SELECT v.i, CASE {tests} END FROM (VALUES {rows}) AS v(i, {names}) \
         WHERE CASE {tests} END IS NOT NULL"
    );
    let result = session.execute_bound(&sql, &params, batch.len()).await?;
    Ok(result
        .rows
        .iter()
        .filter_map(|row| {
            let index: usize = match row.first() {
                Some(super::Cell::Value(v)) => v.parse().ok()?,
                _ => return None,
            };
            let k: usize = match row.get(1) {
                Some(super::Cell::Value(v)) => v.parse().ok()?,
                _ => return None,
            };
            let (name, data_type) = &columns[checked[k]];
            let shown = match &batch[index].values[checked[k]] {
                Param::Text(text) | Param::Number(text) => text.clone(),
                Param::Null => String::new(),
            };
            Some((
                index,
                format!("“{shown}” isn't a valid {data_type} for {name}."),
            ))
        })
        .collect())
}

fn value(engine: Engine, field: Option<String>, options: &ImportOptions) -> Param {
    match field {
        None => Param::Null,
        Some(text) if text.is_empty() && options.empty_as_null => Param::Null,
        Some(text) => match reader::engine_boolean(engine, &text) {
            Some(number) => Param::Text(number.to_owned()),
            None => Param::Text(text),
        },
    }
}

#[cfg(test)]
mod tests;

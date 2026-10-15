//! Copying and exporting a table by re-reading it from the server.
//!
//! A table tab no longer keeps its rows, so copy and export have to fetch
//! them again. They stream: one page at a time, written straight out, so
//! exporting a million rows costs one page of memory. That is the whole reason
//! this module exists rather than a `SELECT` into a buffer.

use crate::{
    database::{
        Cell, ConnectParams, Engine,
        filter::RowFilter,
        paging::{PageOrder, page_sql},
        quote,
        values::ColumnMeta,
    },
    errors::AppError,
    queries::QueryTabHandle,
    results::csv_field,
};
use std::io::Write;

/// Rows read from the server per page while streaming.
const PAGE: usize = 500;

/// How rows are written out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferFormat {
    Tsv,
    Csv,
    Json,
    Markdown,
    /// `INSERT INTO t (cols) VALUES (...), (...);`
    SqlInsert,
    /// `UPDATE t SET c = v WHERE key = k;` — needs the primary key.
    SqlUpdate,
}

impl TransferFormat {
    /// Line ending: RFC 4180 for CSV, newlines for the rest.
    fn line_end(self) -> &'static str {
        match self {
            TransferFormat::Csv => "\r\n",
            _ => "\n",
        }
    }

    fn is_delimited(self) -> bool {
        matches!(self, TransferFormat::Csv | TransferFormat::Tsv)
    }
}

/// What to read and which rows of it.
#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferRequest {
    pub database: String,
    #[serde(default)]
    pub schema: String,
    pub name: String,
    #[serde(default)]
    pub filters: Vec<RowFilter>,
    /// The table's key, used for the tie-breaker and for `UPDATE` statements.
    #[serde(default)]
    pub primary_key: Vec<String>,
    #[serde(default)]
    pub order: Option<PageOrder>,
    /// Column names in display order; the header and SQL statements use them.
    #[serde(default)]
    pub columns: Vec<String>,
    /// Column indexes to include, in order. Empty means every column.
    #[serde(default)]
    pub columns_scope: Vec<usize>,
    /// Row positions to include, inclusive start, exclusive end.
    #[serde(default)]
    pub rows: Option<(usize, usize)>,
    #[serde(default)]
    pub include_header: bool,
}

/// What was written.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferSummary {
    pub rows: usize,
}

/// The column indexes a request covers, in order.
fn selected_columns(request: &TransferRequest) -> Vec<usize> {
    if request.columns_scope.is_empty() {
        return (0..request.columns.len()).collect();
    }
    request
        .columns_scope
        .iter()
        .copied()
        .filter(|index| *index < request.columns.len())
        .collect()
}

/// Streams a table into `out`, one page at a time.
///
/// `limit` caps how many rows are read, so a copy of a very large table cannot
/// quietly become a gigabyte of text; an export passes `None` and reads it all.
pub async fn stream_table(
    tab: &QueryTabHandle,
    params: &ConnectParams,
    request: &TransferRequest,
    format: TransferFormat,
    limit: Option<usize>,
    out: &mut impl Write,
) -> Result<TransferSummary, AppError> {
    let columns = selected_columns(request);
    if columns.is_empty() {
        return Err(AppError::new(
            "nothing_to_export",
            "This table has no columns to copy.",
        ));
    }
    let (start, end) = request.rows.unwrap_or((0, usize::MAX));
    let total = limit.map(|cap| (start + cap).min(end)).unwrap_or(end);

    let names: Vec<&str> = columns
        .iter()
        .map(|i| request.columns[*i].as_str())
        .collect();
    let line_end = format.line_end();
    if format == TransferFormat::Json {
        writeln!(out, "{{\"rows\":[").ok();
    } else if format == TransferFormat::Markdown && request.include_header {
        write!(
            out,
            "| {} |{line_end}",
            names
                .iter()
                .map(|name| md_cell(name))
                .collect::<Vec<_>>()
                .join(" | ")
        )
        .ok();
        write!(
            out,
            "|{}|{line_end}",
            names.iter().map(|_| " --- ").collect::<Vec<_>>().join("|")
        )
        .ok();
    } else if format.is_delimited() && request.include_header {
        let separator = if format == TransferFormat::Csv {
            ","
        } else {
            "\t"
        };
        write!(
            out,
            "{}{line_end}",
            names
                .iter()
                .map(|name| field(name, format))
                .collect::<Vec<_>>()
                .join(separator)
        )
        .ok();
    } else if format == TransferFormat::SqlInsert && request.include_header {
        writeln!(
            out,
            "-- Columns: {}",
            names
                .iter()
                .map(|name| quote::ident(params.engine, name))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .ok();
    }

    let mut written = 0usize;
    let mut offset = start;
    while offset < total {
        let want = PAGE.min(total - offset);
        let sql = page_sql(
            params.engine,
            &request.database,
            &request.schema,
            &request.name,
            "",
            request.order.as_ref(),
            &request.primary_key,
            offset,
            want,
        );
        let result = tab
            .run(params, Some(request.database.clone()), &sql, want, None)
            .await?;
        let fetched = result.rows.len();
        for row in &result.rows {
            write_row(
                out,
                params,
                request,
                format,
                columns.as_slice(),
                row,
                written,
            )?;
            written += 1;
        }
        if fetched < want {
            break;
        }
        offset += fetched;
    }
    if format == TransferFormat::Json {
        writeln!(out, "]}}").ok();
    }
    Ok(TransferSummary { rows: written })
}

/// One row, in the requested format.
fn write_row(
    out: &mut impl Write,
    params: &ConnectParams,
    request: &TransferRequest,
    format: TransferFormat,
    columns: &[usize],
    row: &[Cell],
    index: usize,
) -> Result<(), AppError> {
    let engine = params.engine;
    let line_end = format.line_end();
    match format {
        TransferFormat::Csv | TransferFormat::Tsv => {
            let separator = if format == TransferFormat::Csv {
                ","
            } else {
                "\t"
            };
            let line: Vec<String> = columns
                .iter()
                .map(|c| field(&text(&row[*c]), format))
                .collect();
            write!(out, "{}{line_end}", line.join(separator)).ok();
        }
        TransferFormat::Markdown => {
            let line: Vec<String> = columns.iter().map(|c| md_cell(&text(&row[*c]))).collect();
            write!(out, "| {} |{line_end}", line.join(" | ")).ok();
        }
        TransferFormat::Json => {
            if index > 0 {
                writeln!(out, ",").ok();
            }
            let values: Vec<String> = columns
                .iter()
                .map(|c| match &row[*c] {
                    Cell::Null => "null".to_owned(),
                    Cell::Value(value) => json_string(value),
                    Cell::Binary { preview, .. } => json_string(&format!("0x{preview}")),
                })
                .collect();
            writeln!(out, "  {{{}}}", values.join(",")).ok();
        }
        TransferFormat::SqlInsert => {
            let values: Vec<String> = columns.iter().map(|c| literal(&row[*c], engine)).collect();
            write!(
                out,
                "INSERT INTO {} ({}) VALUES ({});{line_end}",
                quote::table(engine, &request.database, &request.schema, &request.name),
                columns
                    .iter()
                    .map(|c| quote::ident(engine, &request.columns[*c]))
                    .collect::<Vec<_>>()
                    .join(", "),
                values.join(", "),
            )
            .ok();
        }
        TransferFormat::SqlUpdate => {
            // The selected columns that make up the key, as indexes.
            let key: Vec<usize> = request
                .primary_key
                .iter()
                .filter_map(|name| columns.iter().position(|c| request.columns[*c] == *name))
                .map(|position| columns[position])
                .collect();
            if key.is_empty() {
                return Err(AppError::new(
                    "sql_update_needs_key",
                    "An UPDATE export needs the table's primary key.",
                ));
            }
            let assignments: Vec<String> = columns
                .iter()
                .filter(|c| !key.contains(c))
                .map(|c| {
                    format!(
                        "{} = {}",
                        quote::ident(engine, &request.columns[*c]),
                        literal(&row[*c], engine)
                    )
                })
                .collect();
            let condition: Vec<String> = key
                .iter()
                .map(|c| {
                    format!(
                        "{} = {}",
                        quote::ident(engine, &request.columns[*c]),
                        literal(&row[*c], engine)
                    )
                })
                .collect();
            write!(
                out,
                "UPDATE {} SET {} WHERE {};{line_end}",
                quote::table(engine, &request.database, &request.schema, &request.name),
                assignments.join(", "),
                condition.join(" AND "),
            )
            .ok();
        }
    }
    Ok(())
}

/// A cell as the grid shows it, which is what copy and export have always used.
fn text(cell: &Cell) -> String {
    match cell {
        Cell::Null => String::new(),
        Cell::Value(value) => value.clone(),
        Cell::Binary { preview, .. } => format!("0x{preview}"),
    }
}

fn field(value: &str, format: TransferFormat) -> String {
    match format {
        TransferFormat::Csv => csv_field(value),
        // Tabs and newlines would break the grid shape when pasted.
        _ => value.replace(['\t', '\n', '\r'], " "),
    }
}

/// A cell as a SQL literal, so the statements run on the same engine.
fn literal(cell: &Cell, engine: Engine) -> String {
    match cell {
        Cell::Null => "NULL".to_owned(),
        Cell::Value(value) => quote::literal(engine, value),
        Cell::Binary { preview, .. } => format!("X'{}'", preview.to_uppercase()),
    }
}

/// Escapes the pipes and newlines that would break a Markdown table.
fn md_cell(value: &str) -> String {
    value.replace(['|', '\n', '\r'], " ").trim().to_owned()
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Column metadata for a table, read once so the header matches the grid.
pub async fn table_columns(
    tab: &QueryTabHandle,
    params: &ConnectParams,
    request: &TransferRequest,
) -> Result<Vec<ColumnMeta>, AppError> {
    let sql = page_sql(
        params.engine,
        &request.database,
        &request.schema,
        &request.name,
        "",
        None,
        &request.primary_key,
        0,
        1,
    );
    Ok(tab
        .run(params, Some(request.database.clone()), &sql, 1, None)
        .await?
        .columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(values: &[Option<&str>]) -> Vec<Cell> {
        values
            .iter()
            .map(|v| match v {
                Some(text) => Cell::Value((*text).to_owned()),
                None => Cell::Null,
            })
            .collect()
    }

    fn request(columns: &[&str]) -> TransferRequest {
        TransferRequest {
            database: "shop".into(),
            schema: "public".into(),
            name: "orders".into(),
            primary_key: vec!["id".into()],
            columns: columns.iter().map(|c| (*c).to_owned()).collect(),
            ..TransferRequest::default()
        }
    }

    #[test]
    fn csv_quotes_the_same_way_results_do() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(field("a\tb", TransferFormat::Tsv), "a b");
    }

    #[test]
    fn json_escapes_what_would_break_the_document() {
        assert_eq!(json_string("plain"), "\"plain\"");
        assert_eq!(json_string("a\"b"), "\"a\\\"b\"");
        assert_eq!(json_string("a\nb"), "\"a\\nb\"");
        assert_eq!(json_string("a\\b"), "\"a\\\\b\"");
    }

    #[test]
    fn null_and_binary_are_shaped_per_format() {
        // An empty cell in TSV, `null` in JSON, NULL in SQL.
        assert_eq!(text(&Cell::Null), "");
        assert_eq!(
            literal(&Cell::Null, Engine::Postgres),
            "NULL",
            "SQL must be able to set a column back to NULL"
        );
        assert!(literal(&Cell::Value("O'Brien".into()), Engine::Postgres).contains("''"));
        assert!(
            literal(
                &Cell::Binary {
                    bytes: 2,
                    preview: "dead".into(),
                    data: std::sync::Arc::from([0xde, 0xad]),
                },
                Engine::MySql
            )
            .starts_with("X'"),
            "binary is written as bytes, not as text"
        );
    }

    #[test]
    fn markdown_cells_cannot_break_the_table() {
        assert_eq!(md_cell("a|b"), "a b");
        assert_eq!(md_cell("a\nb"), "a b");
        assert_eq!(md_cell("  padded  "), "padded");
    }

    /// Each format written straight out, so the statements are known to be
    /// runnable rather than merely plausible.
    fn params_for_tests() -> ConnectParams {
        ConnectParams {
            engine: Engine::Postgres,
            host: "localhost".into(),
            port: 5432,
            database: Some("shop".into()),
            username: "app".into(),
            password: None,
            tls: crate::database::TlsMode::Verify,
            ca_path: None,
            client_cert: None,
            client_key: None,
            ssh: None,
            connect_to: None,
            read_only: false,
            startup_commands: Vec::new(),
        }
    }

    fn write(format: TransferFormat, request: &TransferRequest, row: &[Cell]) -> String {
        let params = params_for_tests();
        let mut out: Vec<u8> = Vec::new();
        write_row(&mut out, &params, request, format, &[0, 1, 2], row, 0).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn writes_each_format_from_a_row() {
        let request = request(&["id", "name", "total"]);
        let row = row(&[Some("7"), Some("O'Brien"), None]);

        // An apostrophe needs no CSV quoting; only a comma, quote or newline does.
        assert_eq!(write(TransferFormat::Csv, &request, &row), "7,O'Brien,\r\n");
        assert_eq!(write(TransferFormat::Tsv, &request, &row), "7\tO'Brien\t\n");
        assert_eq!(
            write(TransferFormat::Json, &request, &row),
            // Values stay JSON strings: a big integer must not become a
            // lossy float, and null still marks SQL NULL.
            "  {\"7\",\"O'Brien\",null}\n"
        );
        assert_eq!(
            write(TransferFormat::Markdown, &request, &row),
            "| 7 | O'Brien |  |\n"
        );
        assert_eq!(
            write(TransferFormat::SqlInsert, &request, &row),
            "INSERT INTO \"public\".\"orders\" (\"id\", \"name\", \"total\") VALUES ('7', 'O''Brien', NULL);\n"
        );
        // The key becomes the WHERE clause and is left out of SET.
        assert_eq!(
            write(TransferFormat::SqlUpdate, &request, &row),
            "UPDATE \"public\".\"orders\" SET \"name\" = 'O''Brien', \"total\" = NULL WHERE \"id\" = '7';\n"
        );
    }

    #[test]
    fn an_update_export_needs_the_primary_key() {
        let mut request = request(&["id", "name"]);
        request.primary_key.clear();
        assert!(
            write_row(
                &mut Vec::new(),
                &params_for_tests(),
                &request,
                TransferFormat::SqlUpdate,
                &[0, 1],
                &row(&[Some("1"), None]),
                0,
            )
            .is_err(),
            "without a key there is no WHERE clause to write"
        );
    }

    #[test]
    fn selected_columns_defaults_to_every_column() {
        let request = request(&["id", "name"]);
        assert_eq!(selected_columns(&request), [0, 1]);

        let mut scoped = request.clone();
        scoped.columns_scope = vec![1];
        assert_eq!(selected_columns(&scoped), [1]);

        // A stale index from a wider column list is dropped, not panicking.
        let mut stale = request.clone();
        stale.columns_scope = vec![0, 9];
        assert_eq!(selected_columns(&stale), [0]);
    }
}

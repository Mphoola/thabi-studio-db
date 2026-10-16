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
    /// An Excel workbook (.xlsx); written by `XlsxSheet`, not `RowWriter`.
    Xlsx,
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

/// Writes rows in one transfer format: a header, then rows, then a footer.
/// Shared by table streaming and by kept query results, so every copy and
/// export path produces identical output.
pub struct RowWriter {
    format: TransferFormat,
    engine: Engine,
    /// The qualified, quoted table the SQL formats address.
    table: String,
    /// Names of the columns written, in order.
    names: Vec<String>,
    /// Positions (within `names`) of the primary key, for UPDATE.
    key: Vec<usize>,
    include_header: bool,
    written: usize,
}

impl RowWriter {
    pub fn new(
        format: TransferFormat,
        engine: Engine,
        table: String,
        names: Vec<String>,
        key: Vec<usize>,
        include_header: bool,
    ) -> Self {
        Self {
            format,
            engine,
            table,
            names,
            key,
            include_header,
            written: 0,
        }
    }

    pub fn rows(&self) -> usize {
        self.written
    }

    pub fn header(&mut self, out: &mut impl Write) -> Result<(), AppError> {
        let line_end = self.format.line_end();
        let names = &self.names;
        let result = match self.format {
            TransferFormat::Xlsx => return Err(not_text()),
            TransferFormat::Json => write!(out, "["),
            TransferFormat::SqlUpdate if self.key.is_empty() => {
                return Err(AppError::new(
                    "sql_update_needs_key",
                    "An UPDATE export needs the table's primary key.",
                ));
            }
            _ if !self.include_header => Ok(()),
            TransferFormat::Markdown => write!(
                out,
                "| {} |{line_end}|{}|{line_end}",
                names
                    .iter()
                    .map(|n| md_cell(n))
                    .collect::<Vec<_>>()
                    .join(" | "),
                names.iter().map(|_| " --- ").collect::<Vec<_>>().join("|")
            ),
            TransferFormat::Csv | TransferFormat::Tsv => write!(
                out,
                "{}{line_end}",
                names
                    .iter()
                    .map(|n| field(n, self.format))
                    .collect::<Vec<_>>()
                    .join(self.separator())
            ),
            TransferFormat::SqlInsert | TransferFormat::SqlUpdate => Ok(()),
        };
        result.map_err(write_failed)
    }

    fn separator(&self) -> &'static str {
        if self.format == TransferFormat::Csv {
            ","
        } else {
            "\t"
        }
    }

    /// One row; `cells` are the written columns, in `names` order.
    pub fn row(&mut self, out: &mut impl Write, cells: &[&Cell]) -> Result<(), AppError> {
        let engine = self.engine;
        let line_end = self.format.line_end();
        let ident = |i: usize| quote::ident(engine, &self.names[i]);
        let result = match self.format {
            TransferFormat::Xlsx => return Err(not_text()),
            TransferFormat::Csv | TransferFormat::Tsv => write!(
                out,
                "{}{line_end}",
                cells
                    .iter()
                    .map(|c| field(&c.export_text(), self.format))
                    .collect::<Vec<_>>()
                    .join(self.separator())
            ),
            TransferFormat::Markdown => write!(
                out,
                "| {} |{line_end}",
                cells
                    .iter()
                    .map(|c| md_cell(&c.export_text()))
                    .collect::<Vec<_>>()
                    .join(" | ")
            ),
            TransferFormat::Json => {
                // Values stay strings: a big integer must not become a lossy
                // float. NULL is null; binary is 0x-prefixed hex.
                let members: Vec<String> = cells
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let value = match c {
                            Cell::Null => "null".to_owned(),
                            other => json_string(&other.export_text()),
                        };
                        format!("{}:{value}", json_string(&self.names[i]))
                    })
                    .collect();
                let lead = if self.written == 0 { "\n" } else { ",\n" };
                write!(out, "{lead}  {{{}}}", members.join(","))
            }
            TransferFormat::SqlInsert => write!(
                out,
                "INSERT INTO {} ({}) VALUES ({});{line_end}",
                self.table,
                (0..cells.len()).map(ident).collect::<Vec<_>>().join(", "),
                cells
                    .iter()
                    .map(|c| literal(c, engine))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            TransferFormat::SqlUpdate => {
                let pair = |i: usize| format!("{} = {}", ident(i), literal(cells[i], engine));
                let set: Vec<String> = (0..cells.len())
                    .filter(|i| !self.key.contains(i))
                    .map(pair)
                    .collect();
                let condition: Vec<String> = self.key.iter().map(|i| pair(*i)).collect();
                if set.is_empty() {
                    Ok(())
                } else {
                    write!(
                        out,
                        "UPDATE {} SET {} WHERE {};{line_end}",
                        self.table,
                        set.join(", "),
                        condition.join(" AND ")
                    )
                }
            }
        };
        self.written += 1;
        result.map_err(write_failed)
    }

    pub fn finish(&mut self, out: &mut impl Write) -> Result<(), AppError> {
        if self.format == TransferFormat::Json {
            let end = if self.written == 0 { "]\n" } else { "\n]\n" };
            write!(out, "{end}").map_err(write_failed)?;
        }
        Ok(())
    }
}

fn not_text() -> AppError {
    AppError::new(
        "not_text",
        "Excel is a file format: export to .xlsx rather than copying it.",
    )
}

fn write_failed(error: std::io::Error) -> AppError {
    AppError::new("export_failed", "The rows couldn't be written.")
        .with_detail(Some(error.to_string()))
}

/// Reads a table's requested rows page by page, with its filters, handing
/// each row's selected cells to `visit`. Pages are read on the tab's session
/// without being kept, so a long export doesn't push the tab's own results
/// out. Returns how many rows were visited.
async fn walk_table(
    tab: &QueryTabHandle,
    params: &ConnectParams,
    request: &TransferRequest,
    limit: Option<usize>,
    mut visit: impl FnMut(&[ColumnMeta], &[&Cell]) -> Result<(), AppError>,
) -> Result<usize, AppError> {
    let columns = selected_columns(request);
    if columns.is_empty() {
        return Err(AppError::new(
            "nothing_to_export",
            "This table has no columns to copy.",
        ));
    }
    let engine = params.engine;
    let filter = super::filter::where_clause(engine, &request.filters)?;
    let (start, end) = request.rows.unwrap_or((0, usize::MAX));
    let total = limit.map(|cap| (start + cap).min(end)).unwrap_or(end);

    let mut visited = 0usize;
    let mut offset = start;
    while offset < total {
        let want = PAGE.min(total - offset);
        let sql = page_sql(
            engine,
            &request.database,
            &request.schema,
            &request.name,
            &filter,
            request.order.as_ref(),
            &request.primary_key,
            offset,
            want,
        );
        let result = tab
            .with_session(params, Some(request.database.clone()), async |s| {
                s.execute(&sql, want).await
            })
            .await?;
        let metas: Vec<ColumnMeta> = columns
            .iter()
            .filter_map(|c| result.columns.get(*c).cloned())
            .collect();
        let fetched = result.rows.len();
        for row in &result.rows {
            let cells: Vec<&Cell> = columns.iter().map(|c| &row[*c]).collect();
            visit(&metas, &cells)?;
            visited += 1;
        }
        if fetched < want {
            break;
        }
        offset += fetched;
    }
    Ok(visited)
}

/// The selected column names, and the positions of the key among them.
fn names_and_key(request: &TransferRequest) -> (Vec<String>, Vec<usize>) {
    let names: Vec<String> = selected_columns(request)
        .iter()
        .map(|i| request.columns[*i].clone())
        .collect();
    let key = request
        .primary_key
        .iter()
        .filter_map(|name| names.iter().position(|n| n == name))
        .collect();
    (names, key)
}

/// Streams a table into `out`, one page at a time, with its filters applied.
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
    let engine = params.engine;
    let (names, key) = names_and_key(request);
    let mut writer = RowWriter::new(
        format,
        engine,
        quote::table(engine, &request.database, &request.schema, &request.name),
        names,
        key,
        request.include_header,
    );
    writer.header(out)?;
    walk_table(tab, params, request, limit, |_, cells| {
        writer.row(out, cells)
    })
    .await?;
    writer.finish(out)?;
    Ok(TransferSummary {
        rows: writer.rows(),
    })
}

/// Exports a table to an Excel workbook at `path`, streaming rows so memory
/// stays flat (constant-memory mode writes each row as it comes).
pub async fn export_table_xlsx(
    tab: &QueryTabHandle,
    params: &ConnectParams,
    request: &TransferRequest,
    path: &std::path::Path,
) -> Result<TransferSummary, AppError> {
    let (names, _) = names_and_key(request);
    // A workbook always gets its header row: it names the columns.
    let mut sheet = XlsxSheet::new(&names, true)?;
    let rows = walk_table(tab, params, request, None, |metas, cells| {
        sheet.row(metas, cells)
    })
    .await?;
    sheet.save(path)?;
    Ok(TransferSummary { rows })
}

/// An Excel sheet written a row at a time.
pub struct XlsxSheet {
    workbook: rust_xlsxwriter::Workbook,
    next_row: u32,
}

/// Excel's last row; a sheet can't hold more.
const XLSX_MAX_ROWS: u32 = 1_048_576;

fn xlsx_failed(error: rust_xlsxwriter::XlsxError) -> AppError {
    AppError::new("export_failed", "The Excel file couldn't be written.")
        .with_detail(Some(error.to_string()))
}

impl XlsxSheet {
    pub fn new(names: &[String], header: bool) -> Result<Self, AppError> {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let sheet = workbook.add_worksheet_with_constant_memory();
        let mut next_row = 0;
        if header {
            let bold = rust_xlsxwriter::Format::new().set_bold();
            for (col, name) in names.iter().enumerate() {
                sheet
                    .write_string_with_format(0, col as u16, name, &bold)
                    .map_err(xlsx_failed)?;
            }
            next_row = 1;
        }
        Ok(Self { workbook, next_row })
    }

    /// One row. Numbers from numeric columns become Excel numbers when that
    /// keeps every digit (15 significant digits); anything else stays text,
    /// so large integers and precise decimals aren't rounded. NULL is empty.
    pub fn row(&mut self, metas: &[ColumnMeta], cells: &[&Cell]) -> Result<(), AppError> {
        if self.next_row >= XLSX_MAX_ROWS {
            return Err(AppError::new(
                "export_too_large",
                "Excel sheets hold at most 1,048,576 rows. Export as CSV instead, or filter the table first.",
            ));
        }
        let row = self.next_row;
        let sheet = self.workbook.worksheet_from_index(0).map_err(xlsx_failed)?;
        for (col, cell) in cells.iter().enumerate() {
            let col = col as u16;
            match cell {
                Cell::Null => {}
                Cell::Value(text) => {
                    let numeric = metas
                        .get(col as usize)
                        .is_some_and(|m| crate::results::is_numeric_type(&m.type_name));
                    match exact_number(text).filter(|_| numeric) {
                        Some(number) => sheet.write_number(row, col, number),
                        None => sheet.write_string(row, col, text),
                    }
                    .map_err(xlsx_failed)?;
                }
                binary @ Cell::Binary { .. } => {
                    sheet
                        .write_string(row, col, binary.export_text().as_ref())
                        .map_err(xlsx_failed)?;
                }
            }
        }
        self.next_row += 1;
        Ok(())
    }

    pub fn save(mut self, path: &std::path::Path) -> Result<(), AppError> {
        self.workbook.save(path).map_err(xlsx_failed)
    }
}

/// The value as a float only when printing it back gives the same digits.
fn exact_number(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    let number: f64 = trimmed.parse().ok()?;
    let digits = trimmed.trim_start_matches(['-', '+']).replace('.', "");
    let significant = digits.trim_start_matches('0').trim_end_matches('0').len();
    (number.is_finite() && significant <= 15).then_some(number)
}

fn field(value: &str, format: TransferFormat) -> String {
    match format {
        TransferFormat::Csv => csv_field(value),
        // Tabs and newlines would break the grid shape when pasted.
        _ => value.replace(['\t', '\n', '\r'], " "),
    }
}

/// A cell as a SQL literal, so the statements run on the same engine.
/// Binary is written in full in each engine's byte-literal syntax.
fn literal(cell: &Cell, engine: Engine) -> String {
    match cell {
        Cell::Null => "NULL".to_owned(),
        Cell::Value(value) => quote::literal(engine, value),
        Cell::Binary { data, .. } => {
            let hex: String = data.iter().map(|b| format!("{b:02X}")).collect();
            match engine {
                Engine::Postgres => format!("'\\x{hex}'::bytea"),
                Engine::MySql => format!("X'{hex}'"),
                Engine::SqlServer => format!("0x{hex}"),
            }
        }
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
        assert_eq!(Cell::Null.export_text(), "");
        assert_eq!(literal(&Cell::Null, Engine::Postgres), "NULL");
        assert!(literal(&Cell::Value("O'Brien".into()), Engine::Postgres).contains("''"));
        // Every byte is written, not just the preview the grid shows.
        let bytes: Vec<u8> = (0..40).collect();
        let binary = Cell::binary(&bytes);
        let mysql = literal(&binary, Engine::MySql);
        assert!(
            mysql.starts_with("X'0001") && mysql.ends_with("2627'"),
            "{mysql}"
        );
        assert_eq!(
            literal(&Cell::binary(&[0xde, 0xad]), Engine::SqlServer),
            "0xDEAD"
        );
        assert_eq!(
            literal(&Cell::binary(&[0xde, 0xad]), Engine::Postgres),
            "'\\xDEAD'::bytea"
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

    fn write_all(
        format: TransferFormat,
        key: Vec<usize>,
        rows: &[Vec<Cell>],
    ) -> Result<String, AppError> {
        let params = params_for_tests();
        let mut writer = RowWriter::new(
            format,
            params.engine,
            quote::table(params.engine, "shop", "public", "orders"),
            vec!["id".into(), "name".into(), "total".into()],
            key,
            true,
        );
        let mut out: Vec<u8> = Vec::new();
        writer.header(&mut out)?;
        for row in rows {
            writer.row(&mut out, &row.iter().collect::<Vec<_>>())?;
        }
        writer.finish(&mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    fn write(format: TransferFormat, row: &[Cell]) -> String {
        let all = write_all(format, vec![0], &[row.to_vec()]).unwrap();
        // Drop the header line(s) to compare just the row.
        match format {
            TransferFormat::Csv | TransferFormat::Tsv => all.split_once('\n').unwrap().1.to_owned(),
            TransferFormat::Markdown => all.splitn(3, '\n').nth(2).unwrap().to_owned(),
            _ => all,
        }
    }

    #[test]
    fn writes_each_format_from_a_row() {
        let row = row(&[Some("7"), Some("O'Brien"), None]);

        // An apostrophe needs no CSV quoting; only a comma, quote or newline does.
        assert_eq!(write(TransferFormat::Csv, &row), "7,O'Brien,\r\n");
        assert_eq!(write(TransferFormat::Tsv, &row), "7\tO'Brien\t\n");
        assert_eq!(
            write(TransferFormat::Json, &row),
            // Objects keyed by column; values stay strings so big integers
            // survive, and null still marks SQL NULL.
            "[\n  {\"id\":\"7\",\"name\":\"O'Brien\",\"total\":null}\n]\n"
        );
        assert_eq!(
            write(TransferFormat::Markdown, &row),
            "| 7 | O'Brien |  |\n"
        );
        assert_eq!(
            write(TransferFormat::SqlInsert, &row),
            "INSERT INTO \"public\".\"orders\" (\"id\", \"name\", \"total\") VALUES ('7', 'O''Brien', NULL);\n"
        );
        // The key becomes the WHERE clause and is left out of SET.
        assert_eq!(
            write(TransferFormat::SqlUpdate, &row),
            "UPDATE \"public\".\"orders\" SET \"name\" = 'O''Brien', \"total\" = NULL WHERE \"id\" = '7';\n"
        );
    }

    #[test]
    fn json_output_is_a_valid_document() {
        let rows = vec![
            row(&[Some("1"), Some("a\"b"), None]),
            row(&[Some("2"), None, Some("3.5")]),
        ];
        let text = write_all(TransferFormat::Json, vec![0], &rows).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed[1]["total"], "3.5");
        assert_eq!(parsed[0]["name"], "a\"b");
        let empty = write_all(TransferFormat::Json, vec![0], &[]).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&empty).unwrap(),
            serde_json::json!([])
        );
    }

    #[test]
    fn an_update_export_needs_the_primary_key() {
        assert!(
            write_all(
                TransferFormat::SqlUpdate,
                vec![],
                &[row(&[Some("1"), None, None])]
            )
            .is_err(),
            "without a key there is no WHERE clause to write"
        );
    }

    #[test]
    fn excel_numbers_only_when_no_digit_is_lost() {
        assert_eq!(exact_number("42"), Some(42.0));
        assert_eq!(exact_number("-1234.50"), Some(-1234.5));
        assert_eq!(exact_number("12345678901234567890"), None);
        assert_eq!(exact_number("0.1234567890123456"), None);
        assert_eq!(exact_number("abc"), None);
    }

    #[test]
    fn writes_an_excel_workbook() {
        let dir = std::env::temp_dir().join(format!("thabi-xlsx-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rows.xlsx");
        let names = vec!["id".to_owned(), "note".to_owned()];
        let metas = vec![
            ColumnMeta {
                name: "id".into(),
                type_name: "INT8".into(),
            },
            ColumnMeta {
                name: "note".into(),
                type_name: "TEXT".into(),
            },
        ];
        let mut sheet = XlsxSheet::new(&names, true).unwrap();
        let (a, b) = (Cell::Value("1".into()), Cell::Null);
        sheet.row(&metas, &[&a, &b]).unwrap();
        sheet.save(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"PK"), "an .xlsx is a zip file");
        std::fs::remove_dir_all(&dir).ok();
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

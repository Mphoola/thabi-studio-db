//! Query results kept in Rust. The frontend only ever holds the page it is
//! showing; sorting, copying and CSV export work on the full fetched set
//! here, so large results never sit in the webview.

use crate::database::{Cell, ColumnMeta, QueryResult};
use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    io::{BufWriter, Write},
    path::Path,
    sync::Mutex,
};

/// Rows sent with the first response; more are fetched as the grid scrolls.
pub const FIRST_PAGE: usize = 200;
/// Largest page the grid may ask for at once.
pub const MAX_PAGE: usize = 2_000;

pub struct StoredResult {
    pub columns: Vec<ColumnMeta>,
    pub rows: Vec<Vec<Cell>>,
    /// Row order for the current sort, cached: (column, descending, order).
    sorted: Mutex<Option<(usize, bool, Vec<u32>)>>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sort {
    pub column: usize,
    pub descending: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TextFormat {
    Tsv,
    Csv,
}

/// Which part of a result to copy or export. `None` means everything.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    /// Row positions in the displayed (sorted) order, inclusive start, exclusive end.
    pub rows: Option<(usize, usize)>,
    /// Column indexes to include, in order.
    pub columns: Option<Vec<usize>>,
    pub include_header: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSummary {
    pub rows: usize,
    pub path: String,
}

/// Numeric-looking types sort by value, not text.
pub fn is_numeric_type(type_name: &str) -> bool {
    let t = type_name.to_ascii_lowercase();
    [
        "int",
        "integer",
        "smallint",
        "bigint",
        "tinyint",
        "mediumint",
        "int2",
        "int4",
        "int8",
        "decimal",
        "numeric",
        "real",
        "float",
        "float4",
        "float8",
        "double",
        "money",
        "smallmoney",
        "i16",
        "i32",
        "i64",
        "u8",
        "f32",
        "f64",
        "intn",
        "floatn",
        "decimaln",
        "numericn",
        "bit",
    ]
    .iter()
    .any(|n| t == *n || t.starts_with(&format!("{n}(")) || t.starts_with(&format!("{n} ")))
}

fn compare(a: &Cell, b: &Cell, numeric: bool) -> Ordering {
    match (a, b) {
        (Cell::Null, Cell::Null) => Ordering::Equal,
        // NULLs sort last in ascending order.
        (Cell::Null, _) => Ordering::Greater,
        (_, Cell::Null) => Ordering::Less,
        (Cell::Value(x), Cell::Value(y)) if numeric => {
            match (x.trim().parse::<f64>(), y.trim().parse::<f64>()) {
                (Ok(p), Ok(q)) => p
                    .partial_cmp(&q)
                    .unwrap_or(Ordering::Equal)
                    .then_with(|| x.cmp(y)),
                _ => x.cmp(y),
            }
        }
        (Cell::Value(x), Cell::Value(y)) => x
            .to_lowercase()
            .cmp(&y.to_lowercase())
            .then_with(|| x.cmp(y)),
        (Cell::Binary { data: x, .. }, Cell::Binary { data: y, .. }) => x.cmp(y),
        (Cell::Binary { .. }, _) => Ordering::Greater,
        (_, Cell::Binary { .. }) => Ordering::Less,
    }
}

impl StoredResult {
    pub fn new(columns: Vec<ColumnMeta>, rows: Vec<Vec<Cell>>) -> Self {
        Self {
            columns,
            rows,
            sorted: Mutex::new(None),
        }
    }

    /// Row indexes in display order for `sort`.
    fn order(&self, sort: Option<Sort>) -> Option<Vec<u32>> {
        let sort = sort.filter(|s| s.column < self.columns.len())?;
        let mut cache = self.sorted.lock().expect("sort cache");
        if let Some((column, descending, order)) = cache.as_ref()
            && *column == sort.column
            && *descending == sort.descending
        {
            return Some(order.clone());
        }
        let numeric = is_numeric_type(&self.columns[sort.column].type_name);
        let mut order: Vec<u32> = (0..self.rows.len() as u32).collect();
        order.sort_by(|&a, &b| {
            let (x, y) = (
                &self.rows[a as usize][sort.column],
                &self.rows[b as usize][sort.column],
            );
            // NULLs stay last in both directions.
            match (x, y) {
                (Cell::Null, Cell::Null) => Ordering::Equal,
                (Cell::Null, _) => Ordering::Greater,
                (_, Cell::Null) => Ordering::Less,
                _ if sort.descending => compare(x, y, numeric).reverse(),
                _ => compare(x, y, numeric),
            }
        });
        *cache = Some((sort.column, sort.descending, order.clone()));
        Some(order)
    }

    /// Rows `offset..offset+limit` in display order.
    pub fn page(&self, offset: usize, limit: usize, sort: Option<Sort>) -> Vec<Vec<Cell>> {
        let limit = limit.min(MAX_PAGE);
        let end = (offset + limit).min(self.rows.len());
        if offset >= end {
            return Vec::new();
        }
        match self.order(sort) {
            Some(order) => order[offset..end]
                .iter()
                .map(|&i| self.rows[i as usize].clone())
                .collect(),
            None => self.rows[offset..end].to_vec(),
        }
    }

    fn selected<'a>(
        &'a self,
        scope: &Scope,
        sort: Option<Sort>,
    ) -> (Vec<usize>, Box<dyn Iterator<Item = &'a Vec<Cell>> + 'a>) {
        let columns: Vec<usize> = scope
            .columns
            .clone()
            .unwrap_or_else(|| (0..self.columns.len()).collect())
            .into_iter()
            .filter(|&c| c < self.columns.len())
            .collect();
        let (start, end) = scope.rows.unwrap_or((0, self.rows.len()));
        let end = end.min(self.rows.len());
        let start = start.min(end);
        let rows: Box<dyn Iterator<Item = &Vec<Cell>>> = match self.order(sort) {
            Some(order) => Box::new(
                order[start..end]
                    .iter()
                    .map(move |&i| &self.rows[i as usize])
                    .collect::<Vec<_>>()
                    .into_iter(),
            ),
            None => Box::new(self.rows[start..end].iter()),
        };
        (columns, rows)
    }

    /// The scoped cells as tab- or comma-separated text, for the clipboard.
    pub fn to_text(&self, scope: &Scope, sort: Option<Sort>, format: TextFormat) -> String {
        let mut out = Vec::new();
        self.write_delimited(&mut out, scope, sort, format, "\n")
            .expect("writing to memory");
        String::from_utf8(out).unwrap_or_default()
    }

    fn write_delimited(
        &self,
        out: &mut impl Write,
        scope: &Scope,
        sort: Option<Sort>,
        format: TextFormat,
        line_end: &str,
    ) -> std::io::Result<usize> {
        let (columns, rows) = self.selected(scope, sort);
        let separator = if format == TextFormat::Csv { "," } else { "\t" };
        let field = |text: &str| match format {
            TextFormat::Csv => csv_field(text),
            // Tabs and newlines would break the grid shape when pasted.
            TextFormat::Tsv => text.replace(['\t', '\n', '\r'], " "),
        };
        if scope.include_header {
            let header: Vec<String> = columns
                .iter()
                .map(|&c| field(&self.columns[c].name))
                .collect();
            write!(out, "{}{line_end}", header.join(separator))?;
        }
        let mut count = 0;
        for row in rows {
            let line: Vec<String> = columns
                .iter()
                .map(|&c| field(&row[c].export_text()))
                .collect();
            write!(out, "{}{line_end}", line.join(separator))?;
            count += 1;
        }
        Ok(count)
    }

    /// Writes the scoped rows as CSV (RFC 4180, CRLF, UTF-8 with BOM so
    /// spreadsheet apps detect the encoding). Streams to disk row by row.
    pub fn export_csv(
        &self,
        path: &Path,
        scope: &Scope,
        sort: Option<Sort>,
    ) -> Result<usize, AppError> {
        let failed = |e: std::io::Error| {
            AppError::new("export", "The CSV file couldn't be written.")
                .with_detail(Some(e.to_string()))
        };
        let file = std::fs::File::create(path).map_err(failed)?;
        let mut out = BufWriter::new(file);
        out.write_all("\u{feff}".as_bytes()).map_err(failed)?;
        let rows = self
            .write_delimited(&mut out, scope, sort, TextFormat::Csv, "\r\n")
            .map_err(failed)?;
        out.flush().map_err(failed)?;
        Ok(rows)
    }
}

impl StoredResult {
    /// The scoped rows as an Excel workbook at `path`.
    pub fn export_xlsx(
        &self,
        path: &Path,
        scope: &Scope,
        sort: Option<Sort>,
    ) -> Result<usize, AppError> {
        let (columns, rows) = self.selected(scope, sort);
        let names: Vec<String> = columns
            .iter()
            .map(|&c| self.columns[c].name.clone())
            .collect();
        let metas: Vec<ColumnMeta> = columns.iter().map(|&c| self.columns[c].clone()).collect();
        let mut sheet = crate::database::transfer::XlsxSheet::new(&names, true)?;
        let mut count = 0;
        for row in rows {
            let cells: Vec<&Cell> = columns.iter().map(|&c| &row[c]).collect();
            sheet.row(&metas, &cells)?;
            count += 1;
        }
        sheet.save(path)?;
        Ok(count)
    }

    /// The scoped rows in any transfer format. SQL formats address `table`
    /// (already quoted) and UPDATE keys on `key` (column names).
    #[allow(clippy::too_many_arguments)] // Mirrors the export command's inputs.
    pub fn write_rows(
        &self,
        out: &mut impl Write,
        scope: &Scope,
        sort: Option<Sort>,
        format: crate::database::transfer::TransferFormat,
        engine: crate::database::Engine,
        table: String,
        key: &[String],
    ) -> Result<usize, AppError> {
        let (columns, rows) = self.selected(scope, sort);
        let names: Vec<String> = columns
            .iter()
            .map(|&c| self.columns[c].name.clone())
            .collect();
        let key = key
            .iter()
            .filter_map(|name| names.iter().position(|n| n == name))
            .collect();
        let mut writer = crate::database::transfer::RowWriter::new(
            format,
            engine,
            table,
            names,
            key,
            scope.include_header,
        );
        writer.header(out)?;
        for row in rows {
            let cells: Vec<&Cell> = columns.iter().map(|&c| &row[c]).collect();
            writer.row(out, &cells)?;
        }
        writer.finish(out)?;
        Ok(writer.rows())
    }
}

/// Quotes a CSV field when it contains a comma, quote, line break or
/// leading/trailing space; quotes inside are doubled.
pub fn csv_field(text: &str) -> String {
    let needs_quotes =
        text.contains([',', '"', '\n', '\r']) || text.starts_with(' ') || text.ends_with(' ');
    if needs_quotes {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

/// Splits a full result into what the frontend gets first and what Rust keeps.
pub fn into_first_page(mut result: QueryResult, result_id: String) -> (QueryResult, StoredResult) {
    let rows = std::mem::take(&mut result.rows);
    result.total_rows = rows.len();
    result.result_id = Some(result_id);
    result.rows = rows.iter().take(FIRST_PAGE).cloned().collect();
    let stored = StoredResult::new(result.columns.clone(), rows);
    (result, stored)
}

/// Checks a path chosen in the save dialog and makes sure it ends in .csv.
pub fn export_path(path: &str) -> Result<std::path::PathBuf, AppError> {
    let mut path = std::path::PathBuf::from(path);
    let invalid = || AppError::new("validation", "Choose where to save the CSV file again.");
    if !path.is_absolute() || path.file_name().is_none() {
        return Err(invalid());
    }
    if !path.parent().is_some_and(Path::is_dir) {
        return Err(invalid());
    }
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("csv"))
    {
        path.set_extension("csv");
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(name: &str, type_name: &str) -> ColumnMeta {
        ColumnMeta {
            name: name.into(),
            type_name: type_name.into(),
        }
    }

    fn v(text: &str) -> Cell {
        Cell::Value(text.into())
    }

    fn sample() -> StoredResult {
        StoredResult::new(
            vec![column("id", "INT8"), column("note", "TEXT")],
            vec![
                vec![v("10"), v("plain")],
                vec![v("9"), v("comma, here")],
                vec![Cell::Null, v("quote \"x\"")],
                vec![v("100"), v("line\nbreak")],
                vec![v("2"), Cell::binary(&[0xde, 0xad])],
            ],
        )
    }

    #[test]
    fn csv_fields_follow_rfc_4180() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("two\nlines"), "\"two\nlines\"");
        assert_eq!(csv_field(" padded"), "\" padded\"");
    }

    #[test]
    fn numeric_columns_sort_by_value_and_nulls_go_last() {
        let result = sample();
        let ids: Vec<_> = result
            .page(
                0,
                10,
                Some(Sort {
                    column: 0,
                    descending: false,
                }),
            )
            .into_iter()
            .map(|row| row[0].export_text().into_owned())
            .collect();
        assert_eq!(ids, vec!["2", "9", "10", "100", ""]);
        let first_desc = &result.page(
            0,
            1,
            Some(Sort {
                column: 0,
                descending: true,
            }),
        )[0][0];
        assert_eq!(first_desc.export_text(), "100");
    }

    #[test]
    fn copies_a_scope_as_tsv_without_breaking_the_grid() {
        let result = sample();
        let text = result.to_text(
            &Scope {
                rows: Some((2, 4)),
                columns: Some(vec![1]),
                include_header: true,
            },
            None,
            TextFormat::Tsv,
        );
        assert_eq!(text, "note\nquote \"x\"\nline break\n");
    }

    #[test]
    fn exports_csv_with_bom_crlf_full_binary_and_empty_nulls() {
        let path = std::env::temp_dir().join(format!("thabi-export-{}.csv", std::process::id()));
        let rows = sample()
            .export_csv(
                &path,
                &Scope {
                    include_header: true,
                    ..Default::default()
                },
                None,
            )
            .unwrap();
        assert_eq!(rows, 5);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert!(text.starts_with('\u{feff}'));
        assert_eq!(
            text.trim_start_matches('\u{feff}'),
            "id,note\r\n10,plain\r\n9,\"comma, here\"\r\n,\"quote \"\"x\"\"\"\r\n100,\"line\nbreak\"\r\n2,0xdead\r\n"
        );
    }

    #[test]
    fn export_paths_must_be_absolute_and_get_a_csv_extension() {
        let dir = std::env::temp_dir();
        let path = export_path(dir.join("orders").to_str().unwrap()).unwrap();
        assert_eq!(path.extension().unwrap(), "csv");
        assert!(export_path("relative.csv").is_err());
        assert!(export_path("/no/such/folder/x.csv").is_err());
    }

    #[test]
    fn first_page_is_bounded_and_the_rest_stays_in_rust() {
        let rows: Vec<Vec<Cell>> = (0..500).map(|i| vec![v(&i.to_string())]).collect();
        let result = QueryResult {
            columns: vec![column("n", "INT4")],
            rows,
            ..Default::default()
        };
        let (first, stored) = into_first_page(result, "r1".into());
        assert_eq!(first.rows.len(), FIRST_PAGE);
        assert_eq!(first.total_rows, 500);
        assert_eq!(stored.page(490, 100, None).len(), 10);
    }
}

//! Reading a CSV, TSV or JSON file as records, one at a time, so a file of
//! any size is read with flat memory. Each record carries the line it
//! started on, for reporting a row the server refused.

use super::super::Engine;
use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Format {
    Csv,
    /// A JSON array of objects, or one object per line (JSON Lines).
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Encoding {
    Utf8,
    /// ISO-8859-1 / Windows-1252 style: each byte is one character.
    Latin1,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadOptions {
    pub format: Format,
    /// For CSV: "," ";" "\t" "|"; empty means detect from the first line.
    #[serde(default)]
    pub delimiter: String,
    #[serde(default = "double_quote")]
    pub quote: String,
    #[serde(default = "yes")]
    pub header: bool,
    #[serde(default = "utf8")]
    pub encoding: Encoding,
}

fn double_quote() -> String {
    "\"".into()
}
fn yes() -> bool {
    true
}
fn utf8() -> Encoding {
    Encoding::Utf8
}

/// One row of the file: where it started, and its fields (None = JSON null).
pub struct Record {
    pub line: u64,
    pub fields: Vec<Option<String>>,
}

fn unreadable(message: impl Into<String>) -> AppError {
    AppError::new("import_file", message)
}

fn decode(bytes: &[u8], encoding: Encoding, line: u64) -> Result<String, AppError> {
    match encoding {
        Encoding::Latin1 => Ok(bytes.iter().map(|&b| b as char).collect()),
        Encoding::Utf8 => String::from_utf8(bytes.to_vec()).map_err(|_| {
            unreadable(format!(
                "Line {line} isn't UTF-8. Try the Latin-1 encoding instead."
            ))
        }),
    }
}

/// The delimiter a CSV's first line most likely uses: the one of `,` `;`
/// tab `|` that appears most, outside quotes.
pub fn detect_delimiter(first_line: &str) -> u8 {
    let mut counts = [(b',', 0usize), (b';', 0), (b'\t', 0), (b'|', 0)];
    let mut quoted = false;
    for byte in first_line.bytes() {
        if byte == b'"' {
            quoted = !quoted;
        } else if !quoted {
            for (candidate, count) in counts.iter_mut() {
                if byte == *candidate {
                    *count += 1;
                }
            }
        }
    }
    counts
        .iter()
        .max_by_key(|(_, count)| *count)
        .filter(|(_, count)| *count > 0)
        .map(|(byte, _)| *byte)
        .unwrap_or(b',')
}

fn single_byte(text: &str, what: &str) -> Result<u8, AppError> {
    let text = if text == "\\t" { "\t" } else { text };
    match text.as_bytes() {
        [byte] => Ok(*byte),
        _ => Err(AppError::new(
            "validation",
            format!("The {what} must be one character."),
        )),
    }
}

/// A reader over a file's records, and the column names it found.
pub struct Reader {
    pub columns: Vec<String>,
    pub delimiter: u8,
    inner: Inner,
    encoding: Encoding,
    total_bytes: u64,
}

enum Inner {
    Csv(csv::Reader<BufReader<File>>),
    JsonLines {
        lines: std::io::Lines<BufReader<File>>,
        line: u64,
        read: u64,
    },
    JsonArray {
        rows: std::vec::IntoIter<serde_json::Map<String, serde_json::Value>>,
        index: u64,
        count: u64,
    },
}

impl Reader {
    pub fn open(path: &str, options: &ReadOptions) -> Result<Self, AppError> {
        let open = || File::open(path).map_err(|_| unreadable("That file couldn't be opened."));
        let total_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        match options.format {
            Format::Csv => {
                let mut first = String::new();
                let mut probe = BufReader::new(open()?);
                let _ = probe.read_line(&mut first);
                let delimiter = if options.delimiter.is_empty() {
                    detect_delimiter(&first)
                } else {
                    single_byte(&options.delimiter, "delimiter")?
                };
                let mut file = open()?;
                // A UTF-8 byte order mark would end up in the first name.
                let mut bom = [0u8; 3];
                let skip = file
                    .read(&mut bom)
                    .map(|n| n == 3 && bom == [0xEF, 0xBB, 0xBF])
                    .unwrap_or(false);
                file.rewind()
                    .map_err(|_| unreadable("That file couldn't be read."))?;
                if skip {
                    let _ = file.read(&mut bom);
                }
                let csv = csv::ReaderBuilder::new()
                    .delimiter(delimiter)
                    .quote(single_byte(&options.quote, "quote")?)
                    .has_headers(false)
                    .flexible(true)
                    .from_reader(BufReader::new(file));
                let mut reader = Self {
                    columns: Vec::new(),
                    delimiter,
                    inner: Inner::Csv(csv),
                    encoding: options.encoding,
                    total_bytes,
                };
                if options.header
                    && let Some(first) = reader.next_record()?
                {
                    reader.columns = first
                        .fields
                        .into_iter()
                        .enumerate()
                        .map(|(i, name)| {
                            let name = name.unwrap_or_default().trim().to_owned();
                            if name.is_empty() {
                                format!("column {}", i + 1)
                            } else {
                                name
                            }
                        })
                        .collect();
                }
                // Without a header the columns are numbered by the preview
                // from the first rows' width; the first row is data.
                Ok(reader)
            }
            Format::Json => {
                let mut start = [0u8; 1];
                let mut file = open()?;
                let mut first_char = None;
                while file.read(&mut start).map(|n| n == 1).unwrap_or(false) {
                    if !start[0].is_ascii_whitespace() {
                        first_char = Some(start[0]);
                        break;
                    }
                }
                if first_char == Some(b'[') {
                    let text = std::fs::read_to_string(path)
                        .map_err(|_| unreadable("That file isn't UTF-8 JSON."))?;
                    let rows: Vec<serde_json::Map<String, serde_json::Value>> =
                        serde_json::from_str(&text).map_err(|e| {
                            unreadable(format!(
                                "That isn't a JSON array of objects (line {}).",
                                e.line()
                            ))
                        })?;
                    let mut columns: Vec<String> = Vec::new();
                    for row in rows.iter().take(1000) {
                        for key in row.keys() {
                            if !columns.contains(key) {
                                columns.push(key.clone());
                            }
                        }
                    }
                    let count = rows.len() as u64;
                    Ok(Self {
                        columns,
                        delimiter: b',',
                        inner: Inner::JsonArray {
                            rows: rows.into_iter(),
                            index: 0,
                            count,
                        },
                        encoding: Encoding::Utf8,
                        total_bytes,
                    })
                } else {
                    // JSON Lines: the columns are the keys of the first rows.
                    let mut columns: Vec<String> = Vec::new();
                    for line in BufReader::new(open()?).lines().take(1000) {
                        let line = line.map_err(|_| unreadable("That file isn't UTF-8 JSON."))?;
                        if let Ok(serde_json::Value::Object(map)) = serde_json::from_str(&line) {
                            for key in map.keys() {
                                if !columns.contains(key) {
                                    columns.push(key.clone());
                                }
                            }
                        }
                    }
                    Ok(Self {
                        columns,
                        delimiter: b',',
                        inner: Inner::JsonLines {
                            lines: BufReader::new(open()?).lines(),
                            line: 0,
                            read: 0,
                        },
                        encoding: Encoding::Utf8,
                        total_bytes,
                    })
                }
            }
        }
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// How far into the file the reader is, for progress.
    pub fn position(&self) -> u64 {
        match &self.inner {
            Inner::Csv(csv) => csv.position().byte(),
            Inner::JsonLines { read, .. } => *read,
            // The array was read whole; progress is the share of rows done.
            Inner::JsonArray { index, count, .. } => {
                self.total_bytes.saturating_mul(*index) / (*count).max(1)
            }
        }
    }

    pub fn next_record(&mut self) -> Result<Option<Record>, AppError> {
        match &mut self.inner {
            Inner::Csv(csv) => {
                let mut record = csv::ByteRecord::new();
                let line = csv.position().line();
                let more = csv.read_byte_record(&mut record).map_err(|e| {
                    unreadable(format!(
                        "Line {} couldn't be read as CSV: {e}",
                        e.position().map(|p| p.line()).unwrap_or(line)
                    ))
                })?;
                if !more {
                    return Ok(None);
                }
                let line = record.position().map(|p| p.line()).unwrap_or(line);
                let fields = record
                    .iter()
                    .map(|bytes| decode(bytes, self.encoding, line).map(Some))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Some(Record { line, fields }))
            }
            Inner::JsonLines { lines, line, read } => loop {
                let Some(text) = lines.next() else {
                    return Ok(None);
                };
                *line += 1;
                let text = text.map_err(|_| unreadable("That file isn't UTF-8 JSON."))?;
                *read += text.len() as u64 + 1;
                if text.trim().is_empty() {
                    continue;
                }
                let value: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|_| unreadable(format!("Line {line} isn't a JSON object.")))?;
                let serde_json::Value::Object(map) = value else {
                    return Err(unreadable(format!("Line {line} isn't a JSON object.")));
                };
                return Ok(Some(Record {
                    line: *line,
                    fields: self.columns.iter().map(|c| json_text(map.get(c))).collect(),
                }));
            },
            Inner::JsonArray { rows, index, .. } => {
                let Some(map) = rows.next() else {
                    return Ok(None);
                };
                *index += 1;
                Ok(Some(Record {
                    line: *index,
                    fields: self.columns.iter().map(|c| json_text(map.get(c))).collect(),
                }))
            }
        }
    }
}

/// A JSON value as the text a column is given: strings as they are, numbers
/// and booleans as written, nested values as JSON, null and absent as NULL.
fn json_text(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::Null => None,
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        other => Some(other.to_string()),
    }
}

/// A boolean the engine's column will take: MySQL's are numbers.
pub fn engine_boolean(engine: Engine, text: &str) -> Option<&'static str> {
    match (engine, text) {
        (Engine::MySql, "true") => Some("1"),
        (Engine::MySql, "false") => Some("0"),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    /// The delimiter used, as text ("\t" for tab), for the options to show.
    pub delimiter: String,
    /// A type that would hold each column's sample, for a new table.
    pub suggested_types: Vec<String>,
}

/// The first rows of a file and the columns it has, before anything is sent.
pub fn preview(path: &str, options: &ReadOptions, engine: Engine) -> Result<Preview, AppError> {
    let mut reader = Reader::open(path, options)?;
    let mut rows = Vec::new();
    while rows.len() < 50 {
        match reader.next_record()? {
            Some(record) => rows.push(record.fields),
            None => break,
        }
    }
    let width = rows
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
        .max(reader.columns.len());
    let mut columns = reader.columns.clone();
    while columns.len() < width {
        columns.push(format!("column {}", columns.len() + 1));
    }
    let suggested_types = (0..columns.len())
        .map(|i| {
            suggest_type(
                engine,
                rows.iter().filter_map(|r| r.get(i).cloned().flatten()),
            )
        })
        .collect();
    Ok(Preview {
        columns,
        rows,
        delimiter: if reader.delimiter == b'\t' {
            "\\t".into()
        } else {
            (reader.delimiter as char).to_string()
        },
        suggested_types,
    })
}

/// The narrowest common type that holds every sample value.
pub fn suggest_type(engine: Engine, samples: impl Iterator<Item = String>) -> String {
    let samples: Vec<String> = samples.filter(|s| !s.trim().is_empty()).collect();
    let all =
        |test: fn(&str) -> bool| !samples.is_empty() && samples.iter().all(|s| test(s.trim()));
    let integer = |s: &str| {
        s.parse::<i64>().is_ok() && !s.starts_with('+') && !(s.len() > 1 && s.starts_with('0'))
    };
    // A leading zero ("007", a postcode) is text; "0.5" is a number.
    fn leading_zero(s: &str) -> bool {
        let digits = s.trim_start_matches('-');
        digits.len() > 1 && digits.starts_with('0') && !digits.starts_with("0.")
    }
    let decimal = |s: &str| {
        s.parse::<f64>().is_ok()
            && s.chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-'))
            && !leading_zero(s)
    };
    let boolean = |s: &str| matches!(s.to_ascii_lowercase().as_str(), "true" | "false");
    let date = |s: &str| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok();
    let timestamp = |s: &str| {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").is_ok()
            || chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").is_ok()
    };
    let longest = samples.iter().map(|s| s.chars().count()).max().unwrap_or(0);
    let text = match engine {
        Engine::Postgres => "TEXT".to_owned(),
        Engine::MySql if longest <= 255 => "VARCHAR(255)".to_owned(),
        Engine::MySql => "TEXT".to_owned(),
        Engine::SqlServer if longest <= 255 => "NVARCHAR(255)".to_owned(),
        Engine::SqlServer => "NVARCHAR(MAX)".to_owned(),
    };
    if all(integer) {
        let big = samples.iter().any(|s| {
            s.trim()
                .parse::<i64>()
                .map(|n| n.abs() > i32::MAX as i64)
                .unwrap_or(false)
        });
        return match (engine, big) {
            (_, true) => "BIGINT".into(),
            (Engine::MySql | Engine::SqlServer, false) => "INT".into(),
            (Engine::Postgres, false) => "INTEGER".into(),
        };
    }
    if all(decimal) {
        let scale = samples
            .iter()
            .map(|s| s.split('.').nth(1).map(str::len).unwrap_or(0))
            .max()
            .unwrap_or(0)
            .min(10);
        return match engine {
            Engine::Postgres => format!("NUMERIC(18,{scale})"),
            _ => format!("DECIMAL(18,{scale})"),
        };
    }
    if all(boolean) {
        return match engine {
            Engine::SqlServer => "BIT".into(),
            _ => "BOOLEAN".into(),
        };
    }
    if all(date) {
        return "DATE".into();
    }
    if all(timestamp) {
        return match engine {
            Engine::Postgres => "TIMESTAMP".into(),
            Engine::MySql => "DATETIME".into(),
            Engine::SqlServer => "DATETIME2".into(),
        };
    }
    text
}

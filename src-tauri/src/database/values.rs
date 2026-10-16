use serde::Serialize;
use std::sync::Arc;

/// One cell of a result. Values travel as text so large integers and
/// high-precision decimals are never squeezed through a JavaScript number.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum Cell {
    Null,
    Value(String),
    /// Binary data: total length and a hex preview of the first bytes. The
    /// full bytes stay in Rust for copying and export.
    Binary {
        bytes: usize,
        preview: String,
        /// Every byte as base64 when the value is small enough to send to
        /// the grid (images, short hashes), so it can be previewed and
        /// copied in full; `None` for larger values, which export handles.
        inline: Option<String>,
        #[serde(skip)]
        data: Arc<[u8]>,
    },
}

impl Cell {
    pub const BINARY_PREVIEW_BYTES: usize = 32;
    /// Binary values up to this size travel to the grid whole (base64).
    pub const INLINE_BINARY_BYTES: usize = 64 * 1024;

    pub fn binary(bytes: &[u8]) -> Self {
        let preview = bytes
            .iter()
            .take(Self::BINARY_PREVIEW_BYTES)
            .map(|byte| format!("{byte:02x}"))
            .collect();
        use base64::Engine as _;
        let inline = (bytes.len() <= Self::INLINE_BINARY_BYTES)
            .then(|| base64::engine::general_purpose::STANDARD.encode(bytes));
        Self::Binary {
            bytes: bytes.len(),
            preview,
            inline,
            data: Arc::from(bytes),
        }
    }

    /// Text used when copying or exporting: NULL becomes empty, binary
    /// becomes 0x-prefixed hex of every byte.
    pub fn export_text(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Self::Null => "".into(),
            Self::Value(text) => text.as_str().into(),
            Self::Binary { data, .. } => {
                let mut hex = String::with_capacity(2 + data.len() * 2);
                hex.push_str("0x");
                for byte in data.iter() {
                    hex.push_str(&format!("{byte:02x}"));
                }
                hex.into()
            }
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMeta {
    pub name: String,
    /// The engine's own type name, e.g. "INT8", "VARCHAR", "decimal".
    pub type_name: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResult {
    pub columns: Vec<ColumnMeta>,
    pub rows: Vec<Vec<Cell>>,
    /// Rows changed by INSERT/UPDATE/DELETE, when the driver reports it.
    pub rows_affected: Option<u64>,
    /// True when more rows existed than were kept.
    pub truncated: bool,
    /// Row-returning result sets after the first (they are in `more`).
    pub extra_result_sets: usize,
    /// The second and later row sets of a batch, each a result of its own.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub more: Vec<QueryResult>,
    pub elapsed_ms: u64,
    /// Rows held in Rust for this result (all fetched rows); `rows` above
    /// carries only the first page. Set by query tabs.
    pub total_rows: usize,
    /// Handle for fetching more pages, sorting, copying and exporting.
    pub result_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_cells_keep_length_and_a_short_preview() {
        let data = vec![0xab; 100];
        let Cell::Binary { bytes, preview, .. } = Cell::binary(&data) else {
            panic!("expected binary cell");
        };
        assert_eq!(bytes, 100);
        assert_eq!(preview.len(), Cell::BINARY_PREVIEW_BYTES * 2);
    }

    #[test]
    fn cells_serialise_with_a_kind_tag() {
        assert_eq!(
            serde_json::to_string(&Cell::Value("9007199254740993".into())).unwrap(),
            r#"{"kind":"value","value":"9007199254740993"}"#
        );
        assert_eq!(
            serde_json::to_string(&Cell::Null).unwrap(),
            r#"{"kind":"null"}"#
        );
    }
}

/// A value bound to a placeholder in user SQL.
///
/// The point of a parameter is that the value never becomes part of the
/// statement, so this travels beside the SQL and is bound by the driver. A
/// number is a number rather than text because PostgreSQL refuses to compare a
/// numeric column with a text one, and an implicit cast would hide that. It is
/// carried as its decimal text so no digit is lost on the way (a float would
/// round integers above 2^53); each driver binds it as an exact type.
#[derive(Clone, Debug, PartialEq, Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum Param {
    Text(String),
    Number(String),
    Null,
}

impl Param {
    /// What this parameter is for binding to a text column.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }
}

#[cfg(test)]
mod param_tests {
    use super::*;

    #[test]
    fn a_text_parameter_is_what_was_typed() {
        assert_eq!(Param::Text("it's here".into()).as_text(), Some("it's here"));
    }

    #[test]
    fn a_number_and_null_are_not_text() {
        assert_eq!(Param::Number("42".into()).as_text(), None);
        assert_eq!(Param::Null.as_text(), None);
    }
}

/// Where a driver puts rows as they arrive. Shared with the query tab, so
/// the UI can show progress while a query runs and a cancelled query keeps
/// the rows that had already arrived.
pub type SharedSink = std::sync::Arc<std::sync::Mutex<RowSink>>;

/// Rows of every row set a statement (or batch) returns, each capped at
/// `max_rows`.
#[derive(Debug, Default)]
pub struct RowSink {
    sets: Vec<QueryResult>,
    open: bool,
    max_rows: usize,
    rows_affected: Option<u64>,
    /// Rows received so far across sets, including any past the cap.
    pub received: usize,
}

impl RowSink {
    pub fn shared(max_rows: usize) -> SharedSink {
        std::sync::Arc::new(std::sync::Mutex::new(Self {
            max_rows,
            ..Self::default()
        }))
    }

    /// Whether a row set is being read (its columns are known).
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// A new row set starts with these columns.
    pub fn begin(&mut self, columns: Vec<ColumnMeta>) {
        self.sets.push(QueryResult {
            columns,
            ..QueryResult::default()
        });
        self.open = true;
    }

    /// Whether the open set still has room; when false, the caller can skip
    /// decoding the row (it would only be counted).
    pub fn wants_row(&mut self) -> bool {
        self.received += 1;
        let max = self.max_rows;
        match self.sets.last_mut() {
            Some(set) if set.rows.len() < max => true,
            Some(set) => {
                set.truncated = true;
                false
            }
            None => false,
        }
    }

    pub fn push(&mut self, row: Vec<Cell>) {
        if let Some(set) = self.sets.last_mut() {
            set.rows.push(row);
        }
    }

    /// The open row set is complete.
    pub fn end(&mut self) {
        self.open = false;
    }

    pub fn affected(&mut self, rows: u64) {
        *self.rows_affected.get_or_insert(0) += rows;
    }

    /// The first row set's columns and first `limit` rows, for showing a
    /// result while it is still arriving.
    pub fn preview(&self, limit: usize) -> Option<QueryResult> {
        let first = self.sets.first()?;
        Some(QueryResult {
            columns: first.columns.clone(),
            rows: first.rows.iter().take(limit).cloned().collect(),
            ..QueryResult::default()
        })
    }

    /// Everything received: the first row set, with the others in `more`.
    /// A statement without rows reports rows affected instead.
    pub fn take(&mut self) -> QueryResult {
        let mut sets = std::mem::take(&mut self.sets);
        if sets.is_empty() {
            return QueryResult {
                rows_affected: self.rows_affected,
                ..QueryResult::default()
            };
        }
        let mut first = sets.remove(0);
        first.extra_result_sets = sets.len();
        first.more = sets;
        first
    }
}

#[cfg(test)]
mod sink_tests {
    use super::*;

    fn meta(name: &str) -> ColumnMeta {
        ColumnMeta {
            name: name.into(),
            type_name: "TEXT".into(),
        }
    }

    #[test]
    fn keeps_every_row_set_and_caps_each() {
        let sink = RowSink::shared(2);
        let mut s = sink.lock().unwrap();
        s.begin(vec![meta("a")]);
        for i in 0..3 {
            if s.wants_row() {
                s.push(vec![Cell::Value(i.to_string())]);
            }
        }
        s.end();
        s.begin(vec![meta("b")]);
        if s.wants_row() {
            s.push(vec![Cell::Null]);
        }
        s.end();
        assert_eq!(s.received, 4);
        let result = s.take();
        assert_eq!(result.rows.len(), 2);
        assert!(result.truncated);
        assert_eq!(result.extra_result_sets, 1);
        assert_eq!(result.more[0].columns[0].name, "b");
        assert_eq!(result.more[0].rows.len(), 1);
    }

    #[test]
    fn a_statement_without_rows_reports_rows_affected() {
        let sink = RowSink::shared(10);
        let mut s = sink.lock().unwrap();
        s.affected(3);
        let result = s.take();
        assert!(result.columns.is_empty());
        assert_eq!(result.rows_affected, Some(3));
    }
}

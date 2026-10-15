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
        #[serde(skip)]
        data: Arc<[u8]>,
    },
}

impl Cell {
    pub const BINARY_PREVIEW_BYTES: usize = 32;

    pub fn binary(bytes: &[u8]) -> Self {
        let preview = bytes
            .iter()
            .take(Self::BINARY_PREVIEW_BYTES)
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Self::Binary {
            bytes: bytes.len(),
            preview,
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
    /// Row-returning result sets after the first, which are not kept yet.
    pub extra_result_sets: usize,
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

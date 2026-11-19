use super::reader::{ReadOptions, Reader, detect_delimiter, suggest_type};
use super::*;

fn file(contents: &[u8]) -> (tempfile_path::Path, String) {
    tempfile_path::write(contents)
}

/// A temporary file that is removed when dropped, without another crate.
mod tempfile_path {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    pub struct Path(pub std::path::PathBuf);
    impl Drop for Path {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    pub fn write(contents: &[u8]) -> (Path, String) {
        let path = std::env::temp_dir().join(format!(
            "thabi-import-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::write(&path, contents).unwrap();
        let text = path.to_string_lossy().into_owned();
        (Path(path), text)
    }
}

fn csv(header: bool) -> ReadOptions {
    ReadOptions {
        format: Format::Csv,
        delimiter: String::new(),
        quote: "\"".into(),
        header,
        encoding: Encoding::Utf8,
    }
}

fn all(reader: &mut Reader) -> Vec<(u64, Vec<Option<String>>)> {
    let mut out = Vec::new();
    while let Some(record) = reader.next_record().unwrap() {
        out.push((record.line, record.fields));
    }
    out
}

#[test]
fn reads_quotes_newlines_and_a_bom_with_line_numbers() {
    let (_keep, path) =
        file(b"\xEF\xBB\xBFid;name\n1;\"O'Hara; Jr\"\n2;\"two\nlines\"\n3;na\xC3\xAFve\n");
    let mut reader = Reader::open(&path, &csv(true)).unwrap();
    assert_eq!(
        reader.columns,
        ["id", "name"],
        "the BOM is not part of a name"
    );
    assert_eq!(reader.delimiter, b';');
    let rows = all(&mut reader);
    assert_eq!(
        rows[0],
        (2, vec![Some("1".into()), Some("O'Hara; Jr".into())])
    );
    assert_eq!(rows[1].1[1].as_deref(), Some("two\nlines"));
    assert_eq!(rows[2].0, 5, "the line after a quoted newline");
    assert_eq!(rows[2].1[1].as_deref(), Some("naïve"));
}

#[test]
fn latin1_files_read_and_bad_utf8_says_where() {
    let (_keep, path) = file(b"name\ncaf\xE9\n");
    let mut reader = Reader::open(&path, &csv(true)).unwrap();
    let error = reader.next_record().err().expect("not UTF-8");
    assert!(error.message.contains("Line 2"), "{}", error.message);
    let mut latin = csv(true);
    latin.encoding = Encoding::Latin1;
    let mut reader = Reader::open(&path, &latin).unwrap();
    assert_eq!(all(&mut reader)[0].1[0].as_deref(), Some("café"));
}

#[test]
fn json_arrays_and_lines_become_rows() {
    let (_keep, path) = file(br#"[{"id": 1, "ok": true, "tags": ["a"]}, {"id": 2, "note": null}]"#);
    let options = ReadOptions {
        format: Format::Json,
        ..csv(true)
    };
    let mut reader = Reader::open(&path, &options).unwrap();
    assert_eq!(reader.columns, ["id", "ok", "tags", "note"]);
    let rows = all(&mut reader);
    assert_eq!(
        rows[0].1,
        [
            Some("1".into()),
            Some("true".into()),
            Some("[\"a\"]".into()),
            None
        ]
    );

    let (_keep, path) = file(b"{\"a\": \"x\"}\n\n{\"a\": \"y\", \"b\": 2}\n");
    let mut reader = Reader::open(&path, &options).unwrap();
    assert_eq!(reader.columns, ["a", "b"]);
    let rows = all(&mut reader);
    assert_eq!(rows[1].0, 3, "blank lines still count");
    assert_eq!(rows[1].1, [Some("y".into()), Some("2".into())]);
}

#[test]
fn guesses_delimiters_and_types() {
    assert_eq!(detect_delimiter("a\tb\tc"), b'\t');
    assert_eq!(detect_delimiter("\"a,b\";c;d"), b';');
    assert_eq!(detect_delimiter("single"), b',');
    let types =
        |values: &[&str]| suggest_type(Engine::Postgres, values.iter().map(|v| v.to_string()));
    assert_eq!(types(&["1", "42", ""]), "INTEGER");
    assert_eq!(types(&["1", "9999999999"]), "BIGINT");
    assert_eq!(types(&["1.5", "2"]), "NUMERIC(18,1)");
    assert_eq!(
        types(&["007"]),
        "TEXT",
        "leading zeros are text, like a postcode"
    );
    assert_eq!(types(&["2026-10-20"]), "DATE");
    assert_eq!(types(&["true", "FALSE"]), "BOOLEAN");
    assert_eq!(
        suggest_type(Engine::SqlServer, ["x".to_string()].into_iter()),
        "NVARCHAR(255)"
    );
}

#[test]
fn inserts_bind_every_value() {
    let columns = vec![
        ("id".to_owned(), "integer".to_owned()),
        ("name".to_owned(), "text".to_owned()),
    ];
    assert_eq!(
        insert_sql(Engine::Postgres, "\"public\".\"t\"", &columns, 2),
        "INSERT INTO \"public\".\"t\" (\"id\", \"name\") VALUES (CAST($1 AS integer), CAST($2 AS text)), (CAST($3 AS integer), CAST($4 AS text))"
    );
    assert_eq!(
        insert_sql(Engine::SqlServer, "[dbo].[t]", &columns, 1),
        "INSERT INTO [dbo].[t] ([id], [name]) VALUES (@P1, @P2)"
    );
    assert!(insert_sql(Engine::MySql, "`t`", &columns, 1).ends_with("VALUES (?, ?)"));
}

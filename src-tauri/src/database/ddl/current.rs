//! What a table's columns are now, read from the server before a change.
//!
//! MySQL changes part of a column by restating all of it (`MODIFY`,
//! `CHANGE`), and SQL Server restates its type and nullability together
//! (`ALTER COLUMN`). A restatement built only from what the person changed
//! loses everything else: retyping a `NOT NULL DEFAULT 'new'` column on MySQL
//! made it nullable with no default. So the column is restated from what the
//! server says it is now, with only the asked-for part replaced.
//!
//! PostgreSQL changes one attribute at a time and needs none of this.

use super::super::{Engine, Session, metadata::ObjectStructure, quote, values::Param};
use crate::errors::AppError;
use std::collections::HashMap;

/// The current state of a table's columns, as far as an engine needs it.
#[derive(Debug, Clone, Default)]
pub struct Current {
    /// MySQL: each column's definition after its name, as
    /// `SHOW CREATE TABLE` prints it (`varchar(20) NOT NULL DEFAULT 'new'`).
    pub definitions: HashMap<String, String>,
    /// SQL Server: whether each column allows NULL.
    pub nullable: HashMap<String, bool>,
    /// SQL Server: the name of the default constraint on each column.
    pub default_constraints: HashMap<String, String>,
}

/// Reads what a change needs to know about the table, before building it.
pub async fn read(
    session: &mut Session,
    engine: Engine,
    database: &str,
    schema: &str,
    table: &str,
) -> Result<Current, AppError> {
    let mut current = Current::default();
    match engine {
        Engine::Postgres => {}
        Engine::MySql => {
            let sql = format!(
                "SHOW CREATE TABLE {}",
                quote::table(engine, database, schema, table)
            );
            let result = session.execute(&sql, 1).await?;
            if let Some(super::super::Cell::Value(text)) =
                result.rows.first().and_then(|row| row.get(1))
            {
                current.definitions = mysql_definitions(text);
            }
        }
        Engine::SqlServer => {
            let structure: ObjectStructure = session
                .describe(database, schema, table, super::super::ObjectKind::Table)
                .await?;
            current.nullable = structure
                .columns
                .iter()
                .map(|c| (c.name.clone(), c.nullable))
                .collect();
            let name = format!(
                "{}.{}",
                quote::ident(engine, schema),
                quote::ident(engine, table)
            );
            let result = session
                .execute_bound(
                    "SELECT c.name, dc.name FROM sys.default_constraints dc \
                     JOIN sys.columns c ON c.object_id = dc.parent_object_id \
                     AND c.column_id = dc.parent_column_id \
                     WHERE dc.parent_object_id = OBJECT_ID(@P1)",
                    &[Param::Text(name)],
                    10_000,
                )
                .await?;
            for row in &result.rows {
                if let (
                    Some(super::super::Cell::Value(column)),
                    Some(super::super::Cell::Value(constraint)),
                ) = (row.first(), row.get(1))
                {
                    current
                        .default_constraints
                        .insert(column.clone(), constraint.clone());
                }
            }
        }
    }
    Ok(current)
}

/// Each column's definition from a MySQL `CREATE TABLE`, keyed by name.
pub fn mysql_definitions(create: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in create.lines() {
        let line = line.trim().trim_end_matches(',');
        let Some(rest) = line.strip_prefix('`') else {
            continue; // Keys, constraints and the closing line.
        };
        // A backquote inside a name is doubled.
        let mut name = String::new();
        let mut chars = rest.char_indices().peekable();
        let mut end = None;
        while let Some((i, c)) = chars.next() {
            if c == '`' {
                if chars.peek().is_some_and(|&(_, next)| next == '`') {
                    name.push('`');
                    chars.next();
                } else {
                    end = Some(i + 1);
                    break;
                }
            } else {
                name.push(c);
            }
        }
        if let Some(end) = end {
            out.insert(name, rest[end..].trim().to_owned());
        }
    }
    out
}

/// Splits a definition into words, keeping quoted strings and parenthesised
/// groups whole: `enum('a b','c')` and `DEFAULT 'it''s'` stay one word each.
fn words(definition: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut quote: Option<char> = None;
    let mut depth = 0usize;
    let mut chars = definition.chars().peekable();
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                word.push(c);
                if c == '\\' && q != '`' {
                    if let Some(next) = chars.next() {
                        word.push(next);
                    }
                } else if c == q {
                    if chars.peek() == Some(&q) {
                        word.push(chars.next().unwrap_or(q));
                    } else {
                        quote = None;
                    }
                }
            }
            None => match c {
                '\'' | '"' | '`' => {
                    quote = Some(c);
                    word.push(c);
                }
                '(' => {
                    depth += 1;
                    word.push(c);
                }
                ')' => {
                    depth = depth.saturating_sub(1);
                    word.push(c);
                }
                c if c.is_whitespace() && depth == 0 => {
                    if !word.is_empty() {
                        out.push(std::mem::take(&mut word));
                    }
                }
                _ => word.push(c),
            },
        }
    }
    if !word.is_empty() {
        out.push(word);
    }
    out
}

/// Words that start a column attribute, so whatever comes before the first
/// of them is the type (`int unsigned`, `varchar(20)`).
const ATTRIBUTES: &[&str] = &[
    "CHARACTER",
    "CHARSET",
    "COLLATE",
    "NOT",
    "NULL",
    "DEFAULT",
    "AUTO_INCREMENT",
    "COMMENT",
    "GENERATED",
    "AS",
    "ON",
    "VIRTUAL",
    "STORED",
    "INVISIBLE",
    "VISIBLE",
    "SRID",
    "UNIQUE",
    "PRIMARY",
    "KEY",
    "CHECK",
    "COLUMN_FORMAT",
    "STORAGE",
    "ENGINE_ATTRIBUTE",
    "SECONDARY_ENGINE_ATTRIBUTE",
];

fn is(word: &str, keyword: &str) -> bool {
    word.eq_ignore_ascii_case(keyword)
}

/// A MySQL column definition, with one part changed and the rest kept.
#[derive(Debug, Clone)]
pub struct MySqlColumn {
    words: Vec<String>,
}

impl MySqlColumn {
    pub fn parse(definition: &str) -> Self {
        Self {
            words: words(definition),
        }
    }

    fn type_len(&self) -> usize {
        self.words
            .iter()
            .position(|w| ATTRIBUTES.iter().any(|a| is(w, a)))
            .unwrap_or(self.words.len())
    }

    /// Replaces the type. A character set or collation only means something
    /// for text, so they are dropped when the new type is not text.
    pub fn set_type(&mut self, data_type: &str) {
        let end = self.type_len();
        self.words.splice(0..end, words(data_type));
        let text = {
            let lower = data_type.trim().to_ascii_lowercase();
            [
                "char",
                "varchar",
                "text",
                "tinytext",
                "mediumtext",
                "longtext",
                "enum",
                "set",
            ]
            .iter()
            .any(|t| lower.starts_with(t))
        };
        if !text {
            self.remove_phrase(&["CHARACTER", "SET"], 1);
            self.remove_phrase(&["CHARSET"], 1);
            self.remove_phrase(&["COLLATE"], 1);
        }
    }

    pub fn set_nullable(&mut self, nullable: bool) {
        // NOT NULL with DEFAULT NULL is refused, and means nothing anyway.
        if !nullable
            && let Some(i) = self.words.iter().position(|w| is(w, "DEFAULT"))
            && self.words.get(i + 1).is_some_and(|w| is(w, "NULL"))
        {
            self.words.drain(i..i + 2);
        }
        let word = if nullable { "NULL" } else { "NOT NULL" };
        if let Some(i) = self.find(&["NOT", "NULL"]) {
            self.words.splice(i..i + 2, words(word));
        } else if let Some(i) = self.find(&["NULL"]) {
            self.words.splice(i..i + 1, words(word));
        } else {
            let at = self.after_type_and_charset();
            self.words.splice(at..at, words(word));
        }
    }

    /// Replaces the comment; an empty one removes it.
    pub fn set_comment(&mut self, comment: &str) {
        self.remove_phrase(&["COMMENT"], 1);
        if !comment.trim().is_empty() {
            self.words.push("COMMENT".into());
            self.words
                .push(quote::literal(Engine::MySql, comment.trim()));
        }
    }

    pub fn definition(&self) -> String {
        self.words.join(" ")
    }

    fn find(&self, phrase: &[&str]) -> Option<usize> {
        (0..self.words.len()).find(|&i| {
            // A phrase inside DEFAULT's value is not an attribute, so the
            // word right after DEFAULT is skipped.
            (i == 0 || !is(&self.words[i - 1], "DEFAULT"))
                && phrase
                    .iter()
                    .enumerate()
                    .all(|(k, p)| self.words.get(i + k).is_some_and(|w| is(w, p)))
        })
    }

    fn remove_phrase(&mut self, phrase: &[&str], values: usize) {
        if let Some(i) = self.find(phrase) {
            let end = (i + phrase.len() + values).min(self.words.len());
            self.words.drain(i..end);
        }
    }

    fn after_type_and_charset(&self) -> usize {
        let mut at = self.type_len();
        loop {
            let w = |k: usize| self.words.get(at + k).map(String::as_str).unwrap_or("");
            if is(w(0), "CHARACTER") && is(w(1), "SET") {
                at += 3;
            } else if is(w(0), "CHARSET") || is(w(0), "COLLATE") {
                at += 2;
            } else {
                return at.min(self.words.len());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CREATE: &str = "CREATE TABLE `orders` (\n  `id` int NOT NULL AUTO_INCREMENT,\n  `status` varchar(20) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL DEFAULT 'new' COMMENT 'it''s state',\n  `odd``name` enum('a b','c') DEFAULT NULL,\n  PRIMARY KEY (`id`)\n) ENGINE=InnoDB";

    #[test]
    fn reads_each_column_definition() {
        let defs = mysql_definitions(CREATE);
        assert_eq!(defs["id"], "int NOT NULL AUTO_INCREMENT");
        assert!(defs["status"].starts_with("varchar(20) CHARACTER SET"));
        assert_eq!(defs["odd`name"], "enum('a b','c') DEFAULT NULL");
        assert_eq!(defs.len(), 3);
    }

    #[test]
    fn changes_one_part_and_keeps_the_rest() {
        let defs = mysql_definitions(CREATE);
        let mut status = MySqlColumn::parse(&defs["status"]);
        status.set_type("varchar(40)");
        assert_eq!(
            status.definition(),
            "varchar(40) CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci NOT NULL DEFAULT 'new' COMMENT 'it''s state'"
        );
        status.set_nullable(true);
        assert!(
            status
                .definition()
                .contains("utf8mb4_0900_ai_ci NULL DEFAULT 'new'")
        );
        status.set_comment("");
        assert!(!status.definition().contains("COMMENT"));
        status.set_comment("new note");
        assert!(status.definition().ends_with("COMMENT 'new note'"));

        let mut id = MySqlColumn::parse(&defs["id"]);
        id.set_type("bigint");
        assert_eq!(id.definition(), "bigint NOT NULL AUTO_INCREMENT");

        let mut status = MySqlColumn::parse(&defs["status"]);
        status.set_type("int");
        assert_eq!(
            status.definition(),
            "int NOT NULL DEFAULT 'new' COMMENT 'it''s state'",
            "a character set means nothing on a number"
        );
    }

    #[test]
    fn a_default_that_looks_like_an_attribute_is_left_alone() {
        let mut odd = MySqlColumn::parse("varchar(10) DEFAULT NULL");
        odd.set_nullable(false);
        assert_eq!(odd.definition(), "varchar(10) NOT NULL");
        let mut odd = MySqlColumn::parse("varchar(10) NOT NULL DEFAULT 'NULL'");
        odd.set_nullable(true);
        assert_eq!(odd.definition(), "varchar(10) NULL DEFAULT 'NULL'");
    }
}

//! Splitting a SQL file into statements, for Run SQL File.
//!
//! Handles each engine's own way of grouping statements: MySQL's
//! `DELIMITER` lines (so procedure bodies with `;` stay whole), SQL Server's
//! `GO` batch separator, and PostgreSQL's dollar-quoted bodies. Strings,
//! quoted names and comments never split a statement.

use super::Engine;

/// One statement and the line it starts on (1-based), for error messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptStatement {
    pub line: usize,
    pub sql: String,
}

pub fn split_script(text: &str, engine: Engine) -> Vec<ScriptStatement> {
    let mut out = Vec::new();
    let mut delimiter = ";".to_owned();
    let mut current = String::new();
    let mut start_line = 1usize;
    let mut line = 1usize;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut at_line_start = true;

    let flush = |current: &mut String, start: usize, out: &mut Vec<ScriptStatement>| {
        let sql = current.trim();
        if !sql.is_empty() && !is_only_comments(sql) {
            out.push(ScriptStatement {
                line: start,
                sql: sql.to_owned(),
            });
        }
        current.clear();
    };

    while i < chars.len() {
        if at_line_start {
            // Line-level commands: DELIMITER (MySQL) and GO (SQL Server).
            let end = chars[i..]
                .iter()
                .position(|&c| c == '\n')
                .map_or(chars.len(), |p| i + p);
            let whole: String = chars[i..end].iter().collect();
            let trimmed = whole.trim();
            if engine == Engine::MySql && trimmed.to_ascii_uppercase().starts_with("DELIMITER ") {
                flush(&mut current, start_line, &mut out);
                delimiter = trimmed[10..].trim().to_owned();
                if delimiter.is_empty() {
                    delimiter = ";".into();
                }
                i = end;
                continue;
            }
            if engine == Engine::SqlServer && is_go(trimmed) {
                flush(&mut current, start_line, &mut out);
                i = end;
                continue;
            }
            at_line_start = false;
        }
        let c = chars[i];
        if current.trim().is_empty() && !c.is_whitespace() {
            start_line = line;
        }
        // Comments.
        if c == '-' && chars.get(i + 1) == Some(&'-') || (engine == Engine::MySql && c == '#') {
            while i < chars.len() && chars[i] != '\n' {
                current.push(chars[i]);
                i += 1;
            }
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'*') {
            current.push_str("/*");
            i += 2;
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                if chars[i] == '\n' {
                    line += 1;
                }
                current.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                current.push_str("*/");
                i += 2;
            }
            continue;
        }
        // Quoted text and names.
        let close = match c {
            '\'' => Some('\''),
            '"' => Some('"'),
            '`' if engine == Engine::MySql => Some('`'),
            '[' if engine == Engine::SqlServer => Some(']'),
            _ => None,
        };
        if let Some(close) = close {
            current.push(c);
            i += 1;
            while i < chars.len() {
                let d = chars[i];
                if d == '\n' {
                    line += 1;
                }
                current.push(d);
                i += 1;
                if engine == Engine::MySql && d == '\\' && close != '`' && i < chars.len() {
                    current.push(chars[i]);
                    i += 1;
                    continue;
                }
                if d == close {
                    if chars.get(i) == Some(&close) && close != ']' {
                        current.push(close);
                        i += 1;
                        continue;
                    }
                    break;
                }
            }
            continue;
        }
        // PostgreSQL dollar quoting: $tag$ … $tag$.
        if engine == Engine::Postgres
            && c == '$'
            && let Some(tag) = dollar_tag(&chars, i)
        {
            let tag_chars: Vec<char> = tag.chars().collect();
            current.push_str(&tag);
            i += tag_chars.len();
            while i < chars.len() && !chars[i..].starts_with(&tag_chars) {
                if chars[i] == '\n' {
                    line += 1;
                }
                current.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                current.push_str(&tag);
                i += tag_chars.len();
            }
            continue;
        }
        // The statement delimiter.
        let delimiter_chars: Vec<char> = delimiter.chars().collect();
        if chars[i..].starts_with(&delimiter_chars) {
            flush(&mut current, start_line, &mut out);
            i += delimiter_chars.len();
            continue;
        }
        if c == '\n' {
            line += 1;
            at_line_start = true;
        }
        current.push(c);
        i += 1;
    }
    flush(&mut current, start_line, &mut out);
    out
}

fn is_go(line: &str) -> bool {
    let mut words = line.split_whitespace();
    matches!(words.next(), Some(w) if w.eq_ignore_ascii_case("go"))
        && words
            .next()
            .is_none_or(|count| count.chars().all(|c| c.is_ascii_digit()))
}

/// `$tag$` (or `$$`) starting at `i`, if there is one.
fn dollar_tag(chars: &[char], i: usize) -> Option<String> {
    let mut end = i + 1;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    // `$1` is a parameter, not a tag.
    if end > i + 1 && chars[i + 1].is_ascii_digit() {
        return None;
    }
    (chars.get(end) == Some(&'$')).then(|| chars[i..=end].iter().collect())
}

fn is_only_comments(sql: &str) -> bool {
    sql.lines().all(|l| {
        let l = l.trim();
        l.is_empty() || l.starts_with("--") || l.starts_with('#')
    }) || (sql.starts_with("/*") && sql.ends_with("*/") && !sql[2..sql.len() - 2].contains("*/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sql(statements: &[ScriptStatement]) -> Vec<&str> {
        statements.iter().map(|s| s.sql.as_str()).collect()
    }

    #[test]
    fn splits_on_semicolons_outside_text_and_comments() {
        let out = split_script(
            "SELECT 'a;b';\n-- one; two\nSELECT \"x;y\" /* ; */;\n\nINSERT INTO t VALUES (1)",
            Engine::Postgres,
        );
        assert_eq!(
            sql(&out),
            [
                "SELECT 'a;b'",
                "-- one; two\nSELECT \"x;y\" /* ; */",
                "INSERT INTO t VALUES (1)"
            ]
        );
        assert_eq!(out[2].line, 5, "line numbers point at the statement");
    }

    #[test]
    fn keeps_postgres_function_bodies_whole() {
        let out = split_script(
            "CREATE FUNCTION f() RETURNS int AS $body$ BEGIN RETURN 1; END; $body$ LANGUAGE plpgsql;\nSELECT $1",
            Engine::Postgres,
        );
        assert_eq!(out.len(), 2);
        assert!(out[0].sql.contains("RETURN 1; END;"));
    }

    #[test]
    fn follows_mysql_delimiter_lines() {
        let out = split_script(
            "DELIMITER //\nCREATE PROCEDURE p() BEGIN SELECT 1; SELECT 2; END//\nDELIMITER ;\nSELECT 'it\\'s';",
            Engine::MySql,
        );
        assert_eq!(
            sql(&out),
            [
                "CREATE PROCEDURE p() BEGIN SELECT 1; SELECT 2; END",
                "SELECT 'it\\'s'"
            ]
        );
        assert_eq!(out[0].line, 2);
    }

    #[test]
    fn splits_sql_server_batches_on_go() {
        let out = split_script(
            "CREATE TABLE [a;b] (x int)\nGO\nINSERT INTO [a;b] VALUES (1);\nINSERT INTO [a;b] VALUES (2)\ngo 2\nSELECT 'go'",
            Engine::SqlServer,
        );
        assert_eq!(
            sql(&out),
            [
                "CREATE TABLE [a;b] (x int)",
                "INSERT INTO [a;b] VALUES (1)",
                "INSERT INTO [a;b] VALUES (2)",
                "SELECT 'go'"
            ]
        );
    }
}

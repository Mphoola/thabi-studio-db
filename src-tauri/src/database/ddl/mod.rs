//! DDL the app generates, one shape per engine.
//!
//! Everything a person sees before it runs. The table designer stages changes,
//! this turns them into the statements each engine wants, and the review dialog
//! shows the result — so what is read is what is sent, and the tests here are
//! the promise that they agree.
//!
//! Nothing here reaches a database. It is text, built only from a structure the
//! user filled in and identifiers that go through [`super::quote`], so a table
//! called `order"; DROP TABLE x; --` is one quoted name in every statement.

pub mod alter;
pub mod keys;
pub mod plan;
pub mod table;

pub use alter::Change;
pub use keys::{IndexSpec, KeyChange};

pub use alter::alter_column;
pub use keys::{alter_key, create_index, drop_index};
pub use plan::{
    Atomicity, DdlPreview, IndexAction, TableAction, TablePlan, TableTarget, action_script,
    atomicity, atomicity_note, preview, preview_of, spec_from_structure, table_script,
};
pub use table::{
    ColumnSpec, ForeignKey, TableSpec, create_table, drop_table, rename_table, truncate_table,
};

/// The statement text with every quoted identifier taken out, so a semicolon
/// inside a name is not counted as a statement break. Asserting on a statement
/// that quotes an awkward name needs this: the name is free to contain the very
/// character the check is looking for.
#[cfg(test)]
pub(crate) fn outside_identifiers(sql: &str) -> String {
    let mut out = String::new();
    let mut rest = sql;
    while let Some(open) = rest.find(['"', '`', '[']) {
        out.push_str(&rest[..open]);
        let close = if rest[open..].starts_with('[') {
            "]"
        } else {
            &rest[open..open + 1]
        };
        match rest[open + 1..].find(close) {
            Some(at) => rest = &rest[open + 1 + at + 1..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

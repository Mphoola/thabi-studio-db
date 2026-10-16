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

pub mod table;

pub use table::{
    ColumnSpec, ForeignKey, TableSpec, create_table, drop_table, rename_table, truncate_table,
};

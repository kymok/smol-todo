//! The `taskpond` CLI: a thin command layer over `pond-core`.

pub mod cli;
pub mod output;
pub mod parse;

pub use cli::{run, CliError};

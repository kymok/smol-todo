#![allow(unused_imports)] // TEMPORARY: removed in Task 9 when all handlers are filled in

use crate::output::{CollectionOutput, ItemOutput};
use crate::parse::{parse_color, parse_status, unescape};
use clap::{Parser, Subcommand};
use pond_core::json::to_pretty_sorted;
use pond_core::{StoreError, TaskStore, DEFAULT_GROUP};
use std::io::Write;

#[derive(Debug)]
pub enum CliError {
    Parse(clap::Error),
    Store(StoreError),
    Usage(String),
}

impl From<StoreError> for CliError {
    fn from(e: StoreError) -> Self {
        CliError::Store(e)
    }
}

#[derive(Parser)]
#[command(
    name = "taskpond",
    about = "A small task store CLI",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: TopCommand,
}

#[derive(Subcommand)]
enum TopCommand {
    /// Work with task items
    Item {
        #[command(subcommand)]
        cmd: ItemCommand,
    },
    /// Work with collections
    Collection {
        #[command(subcommand)]
        cmd: CollectionCommand,
    },
}

#[derive(Subcommand)]
enum ItemCommand {
    Create {
        #[arg(short = 'c', long = "collection")]
        collection: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        title: Vec<String>,
    },
    Get {
        #[arg(short = 's', long = "status")]
        status: Option<String>,
        #[arg(short = 'c', long = "collection")]
        collection: Option<String>,
        ids: Vec<String>,
    },
    Update {
        id: String,
        #[arg(short = 'c', long = "collection")]
        collection: Option<String>,
        #[arg(short = 's', long = "status")]
        status: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        title: Vec<String>,
    },
    Note {
        #[command(subcommand)]
        cmd: NoteCommand,
    },
    Delete {
        #[arg(short = 'c', long = "collection")]
        collection: Option<String>,
        ids: Vec<String>,
    },
}

#[derive(Subcommand)]
enum NoteCommand {
    Add {
        id: String,
        #[arg(long = "body")]
        body: String,
    },
    Update {
        id: String,
        #[arg(long = "body")]
        body: String,
    },
    Delete {
        id: String,
    },
}

#[derive(Subcommand)]
enum CollectionCommand {
    List,
    Create {
        name: String,
    },
    Rename {
        old_name: String,
        new_name: String,
    },
    Color {
        name: String,
        color: String,
    },
    Delete {
        name: String,
    },
    Clear {
        name: String,
        #[arg(long = "completed")]
        completed: bool,
    },
}

/// Parse `args` (including the program name at index 0), run the command against
/// `store`, and write JSON output to `out`. Returns `CliError` on failure.
pub fn run(args: &[String], store: &TaskStore, out: &mut dyn Write) -> Result<(), CliError> {
    let cli = Cli::try_parse_from(args).map_err(CliError::Parse)?;
    match cli.command {
        TopCommand::Item { cmd } => run_item(cmd, store, out),
        TopCommand::Collection { cmd } => run_collection(cmd, store, out),
    }
}

#[allow(clippy::match_single_binding)]
fn run_item(cmd: ItemCommand, _store: &TaskStore, _out: &mut dyn Write) -> Result<(), CliError> {
    match cmd {
        _ => Ok(()), // handlers filled in Tasks 5-7
    }
}

#[allow(clippy::match_single_binding)]
fn run_collection(
    cmd: CollectionCommand,
    _store: &TaskStore,
    _out: &mut dyn Write,
) -> Result<(), CliError> {
    match cmd {
        _ => Ok(()), // handlers filled in Tasks 8-9
    }
}

#[cfg(test)]
pub(crate) fn run_capture(args: &[&str], store: &TaskStore) -> (Result<(), CliError>, String) {
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut buf: Vec<u8> = Vec::new();
    let res = run(&argv, store, &mut buf);
    (res, String::from_utf8(buf).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn unknown_command_is_a_parse_error() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let (res, _out) = run_capture(&["taskpond", "bogus"], &store);
        assert!(matches!(res, Err(CliError::Parse(_))));
    }
}

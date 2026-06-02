#![allow(unused_imports)] // TEMPORARY: removed in Task 9 when all handlers are filled in

use crate::output::{CollectionOutput, ItemOutput};
use crate::parse::{parse_color, parse_status, unescape};
use clap::{Parser, Subcommand};
use pond_core::json::to_pretty_sorted;
use pond_core::{StoreError, TaskStore, DEFAULT_COLLECTION, DEFAULT_GROUP};
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

fn run_item(cmd: ItemCommand, store: &TaskStore, out: &mut dyn Write) -> Result<(), CliError> {
    match cmd {
        ItemCommand::Create { collection, title } => {
            // Swift fires "Create requires a title." only when no title tokens are given;
            // a whitespace-only title falls through to store.add, which returns InvalidTitle.
            if title.is_empty() {
                return Err(CliError::Usage("Create requires a title.".to_string()));
            }
            let title = unescape(&title.join(" "));
            let collection = collection.unwrap_or_else(|| DEFAULT_COLLECTION.to_string());
            let item = store.add(
                &title,
                &collection,
                None,
                false,
                pond_core::TaskStatus::Ready,
            )?;
            print_items(out, &[item])
        }
        ItemCommand::Get {
            status,
            collection,
            ids,
        } => {
            if collection.is_some() && !ids.is_empty() {
                return Err(CliError::Store(StoreError::TargetConflict));
            }
            let status = match status {
                Some(s) => Some(parse_status(&s).map_err(CliError::Usage)?),
                None => None,
            };
            let items = store.items(status, collection.as_deref(), &ids, None)?;
            print_items(out, &items)
        }
        _ => Ok(()),
    }
}

fn print_items(out: &mut dyn Write, items: &[pond_core::TaskItem]) -> Result<(), CliError> {
    let outputs: Vec<ItemOutput> = items.iter().map(ItemOutput::from_item).collect();
    let json = to_pretty_sorted(&outputs).map_err(CliError::Store)?;
    writeln!(out, "{json}").map_err(|e| CliError::Usage(e.to_string()))
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

    #[test]
    fn create_then_get_round_trips_json() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let (res, out) = run_capture(
            &["taskpond", "item", "create", "-c", "Inbox", "Buy", "milk"],
            &store,
        );
        assert!(res.is_ok());
        assert!(out.contains("\"title\": \"Buy milk\""));
        assert!(out.contains("\"status\": \"ready\""));
        assert!(out.contains("\"collection\": \"Inbox\""));

        let (res, out) = run_capture(&["taskpond", "item", "get"], &store);
        assert!(res.is_ok());
        assert!(out.contains("Buy milk"));
    }

    #[test]
    fn create_requires_a_title() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let (res, _out) = run_capture(&["taskpond", "item", "create", "-c", "Inbox"], &store);
        assert!(matches!(res, Err(CliError::Usage(m)) if m == "Create requires a title."));
    }

    #[test]
    fn create_whitespace_title_is_invalid() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        // A whitespace-only title is not "no title" — it reaches the store, which
        // rejects it with InvalidTitle (matching Swift).
        let (res, _out) = run_capture(
            &["taskpond", "item", "create", "-c", "Inbox", "   "],
            &store,
        );
        assert!(matches!(
            res,
            Err(CliError::Store(StoreError::InvalidTitle))
        ));
    }

    #[test]
    fn get_rejects_collection_and_ids_together() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add(
                "a",
                "Inbox",
                Some("00000001"),
                false,
                pond_core::TaskStatus::Ready,
            )
            .unwrap();
        let (res, _out) = run_capture(
            &["taskpond", "item", "get", "-c", "Inbox", "00000001"],
            &store,
        );
        assert!(matches!(
            res,
            Err(CliError::Store(StoreError::TargetConflict))
        ));
    }
}

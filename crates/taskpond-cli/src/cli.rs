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
        ItemCommand::Update {
            id,
            collection,
            status,
            title,
        } => {
            let title = if title.is_empty() {
                None
            } else {
                Some(unescape(&title.join(" ")))
            };
            let status = match status {
                Some(s) => Some(parse_status(&s).map_err(CliError::Usage)?),
                None => None,
            };
            let item = store.update(&id, title.as_deref(), collection.as_deref(), status)?;
            print_items(out, &[item])
        }
        ItemCommand::Delete { collection, ids } => {
            let deleted = store.delete_many(&ids, collection.as_deref())?;
            print_items(out, &deleted)
        }
        ItemCommand::Note { cmd } => match cmd {
            NoteCommand::Add { id, body } => {
                let item = store.add_note(&id, &unescape(&body))?;
                print_items(out, &[item])
            }
            NoteCommand::Update { id, body } => {
                let item = store.update_note(&id, &unescape(&body))?;
                print_items(out, &[item])
            }
            NoteCommand::Delete { id } => {
                let item = store.delete_note(&id)?;
                print_items(out, &[item])
            }
        },
    }
}

fn print_items(out: &mut dyn Write, items: &[pond_core::TaskItem]) -> Result<(), CliError> {
    let outputs: Vec<ItemOutput> = items.iter().map(ItemOutput::from_item).collect();
    let json = to_pretty_sorted(&outputs).map_err(CliError::Store)?;
    writeln!(out, "{json}").map_err(|e| CliError::Usage(e.to_string()))
}

fn run_collection(
    cmd: CollectionCommand,
    store: &TaskStore,
    out: &mut dyn Write,
) -> Result<(), CliError> {
    match cmd {
        CollectionCommand::List => {
            let summaries = store.collection_summaries()?;
            print_collections(out, &summaries)
        }
        CollectionCommand::Create { name } => {
            let created = store.create_collection(&unescape(&name), DEFAULT_GROUP)?;
            print_collection_named(out, store, &created)
        }
        CollectionCommand::Rename { old_name, new_name } => {
            let final_name = store.rename_collection(&unescape(&old_name), &unescape(&new_name))?;
            print_collection_named(out, store, &final_name)
        }
        CollectionCommand::Color { name, color } => {
            let color = parse_color(&color).map_err(CliError::Usage)?;
            let summary = store.set_collection_color(&unescape(&name), color)?;
            print_collections(out, &[summary])
        }
        CollectionCommand::Delete { name } => {
            let clean = unescape(&name);
            let summary = store
                .collection_summaries()?
                .into_iter()
                .find(|c| c.name == clean)
                .ok_or_else(|| CliError::Store(StoreError::CollectionNotFound(clean.clone())))?;
            store.delete_collection(&clean)?;
            print_collections(out, &[summary])
        }
        CollectionCommand::Clear { name, completed } => {
            let cleared = store.clear_items(&unescape(&name), completed)?;
            print_items(out, &cleared)
        }
    }
}

fn print_collections(
    out: &mut dyn Write,
    summaries: &[pond_core::CollectionSummary],
) -> Result<(), CliError> {
    let outputs: Vec<CollectionOutput> = summaries
        .iter()
        .map(CollectionOutput::from_summary)
        .collect();
    let json = to_pretty_sorted(&outputs).map_err(CliError::Store)?;
    writeln!(out, "{json}").map_err(|e| CliError::Usage(e.to_string()))
}

/// Look up a collection summary by api name and print it (matches Swift's printCollection).
fn print_collection_named(
    out: &mut dyn Write,
    store: &TaskStore,
    name: &str,
) -> Result<(), CliError> {
    let summary = store
        .collection_summaries()?
        .into_iter()
        .find(|c| c.name == name)
        .ok_or_else(|| CliError::Store(StoreError::CollectionNotFound(name.to_string())))?;
    print_collections(out, &[summary])
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

    #[test]
    fn update_changes_fields() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add(
                "old",
                "Inbox",
                Some("0123abcd"),
                false,
                pond_core::TaskStatus::Ready,
            )
            .unwrap();
        let (res, out) = run_capture(
            &[
                "taskpond",
                "item",
                "update",
                "0123abcd",
                "-s",
                "in-progress",
                "new",
                "title",
            ],
            &store,
        );
        assert!(res.is_ok());
        assert!(out.contains("\"title\": \"new title\""));
        assert!(out.contains("\"status\": \"in-progress\""));
    }

    #[test]
    fn update_requires_a_field() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add(
                "a",
                "Inbox",
                Some("0123abcd"),
                false,
                pond_core::TaskStatus::Ready,
            )
            .unwrap();
        let (res, _out) = run_capture(&["taskpond", "item", "update", "0123abcd"], &store);
        assert!(matches!(
            res,
            Err(CliError::Store(StoreError::MissingUpdate))
        ));
    }

    #[test]
    fn delete_requires_a_target() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let (res, _out) = run_capture(&["taskpond", "item", "delete"], &store);
        assert!(matches!(
            res,
            Err(CliError::Store(StoreError::MissingTarget))
        ));
    }

    #[test]
    fn note_add_update_delete() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add(
                "a",
                "Inbox",
                Some("0123abcd"),
                false,
                pond_core::TaskStatus::Ready,
            )
            .unwrap();

        let (res, out) = run_capture(
            &[
                "taskpond", "item", "note", "add", "0123abcd", "--body", "hello",
            ],
            &store,
        );
        assert!(res.is_ok());
        assert!(out.contains("\"body\": \"hello\""));

        let (res, out) = run_capture(
            &[
                "taskpond", "item", "note", "update", "0123abcd", "--body", "world",
            ],
            &store,
        );
        assert!(res.is_ok());
        assert!(out.contains("\"body\": \"world\""));

        let (res, out) = run_capture(&["taskpond", "item", "note", "delete", "0123abcd"], &store);
        assert!(res.is_ok());
        assert!(!out.contains("\"note\""));
    }

    #[test]
    fn note_add_to_missing_item_errors() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        let (res, _out) = run_capture(
            &["taskpond", "item", "note", "add", "deadbeef", "--body", "x"],
            &store,
        );
        assert!(matches!(res, Err(CliError::Store(StoreError::NotFound(_)))));
    }

    #[test]
    fn collection_create_list_rename_color() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));

        let (res, out) = run_capture(&["taskpond", "collection", "create", "Errands"], &store);
        assert!(res.is_ok());
        assert!(out.contains("\"name\": \"Errands\""));

        let (res, out) = run_capture(
            &["taskpond", "collection", "color", "Errands", "blue"],
            &store,
        );
        assert!(res.is_ok());
        assert!(out.contains("\"color\": \"blue\""));

        let (res, out) = run_capture(
            &["taskpond", "collection", "rename", "Errands", "Personal"],
            &store,
        );
        assert!(res.is_ok());
        assert!(out.contains("\"name\": \"Personal\""));

        let (res, out) = run_capture(&["taskpond", "collection", "list"], &store);
        assert!(res.is_ok());
        assert!(out.contains("Personal"));
    }

    #[test]
    fn collection_color_rejects_bad_color() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store.create_collection("Errands", DEFAULT_GROUP).unwrap();
        let (res, _out) = run_capture(
            &["taskpond", "collection", "color", "Errands", "teal"],
            &store,
        );
        assert!(matches!(res, Err(CliError::Usage(m)) if m.contains("Expected 'gray'")));
    }

    #[test]
    fn collection_delete_prints_predelete_summary() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add(
                "a",
                "Work/A",
                Some("00000001"),
                false,
                pond_core::TaskStatus::Ready,
            )
            .unwrap();
        let (res, out) = run_capture(&["taskpond", "collection", "delete", "Work/A"], &store);
        assert!(res.is_ok());
        assert!(out.contains("\"name\": \"Work/A\""));
        assert!(out.contains("\"totalCount\": 1"));
        // collection is gone afterward
        let (_res, out) = run_capture(&["taskpond", "collection", "list"], &store);
        assert!(!out.contains("Work/A"));
    }

    #[test]
    fn collection_clear_completed_only() {
        let dir = tempdir().unwrap();
        let store = TaskStore::new(dir.path().join("tasks.json"));
        store
            .add(
                "done",
                "Inbox",
                Some("00000001"),
                false,
                pond_core::TaskStatus::Completed,
            )
            .unwrap();
        store
            .add(
                "open",
                "Inbox",
                Some("00000002"),
                false,
                pond_core::TaskStatus::Ready,
            )
            .unwrap();
        let (res, out) = run_capture(
            &["taskpond", "collection", "clear", "Inbox", "--completed"],
            &store,
        );
        assert!(res.is_ok());
        assert!(out.contains("done"));
        assert!(store.items(None, Some("Inbox"), &[], None).unwrap().len() == 1);
    }
}

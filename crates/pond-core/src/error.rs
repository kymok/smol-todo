use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    InvalidTitle,
    InvalidCollection,
    InvalidCollectionGroup,
    DefaultCollection,
    DefaultCollectionGroup,
    InvalidId(String),
    MissingTarget,
    MissingUpdate,
    MissingNoteUpdate,
    TargetConflict,
    NoMatchingTasks,
    NotFound(String),
    NoteNotFound(String),
    CollectionNotFound(String),
    CollectionGroupNotFound(String),
    CollectionConflict(String),
    AmbiguousId(String, Vec<String>),
    DuplicateId(String),
    InvalidNote,
    FileLockFailed(String),
    Io(String),
    Serde(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::InvalidTitle => write!(f, "Task title cannot be empty."),
            StoreError::InvalidCollection => write!(f, "Collection name cannot be empty."),
            StoreError::InvalidCollectionGroup => {
                write!(f, "Collection group name cannot be empty.")
            }
            StoreError::DefaultCollection => write!(
                f,
                "Default collection cannot be renamed, deleted, or moved."
            ),
            StoreError::DefaultCollectionGroup => {
                write!(f, "Default collection group cannot be renamed or deleted.")
            }
            StoreError::InvalidId(id) => write!(f, "Task id '{id}' is invalid."),
            StoreError::MissingTarget => {
                write!(f, "Command requires --collection or at least one id.")
            }
            StoreError::MissingUpdate => {
                write!(f, "Update requires a title, --collection, or --status/-s.")
            }
            StoreError::MissingNoteUpdate => write!(f, "Note update requires --body."),
            StoreError::TargetConflict => write!(f, "Use either --collection or ids, not both."),
            StoreError::NoMatchingTasks => write!(f, "No matching tasks."),
            StoreError::NotFound(id) => write!(f, "No task matches '{id}'."),
            StoreError::NoteNotFound(id) => write!(f, "No note matches '{id}'."),
            StoreError::CollectionNotFound(name) => write!(f, "No collection matches '{name}'."),
            StoreError::CollectionGroupNotFound(name) => {
                write!(f, "No collection group matches '{name}'.")
            }
            StoreError::CollectionConflict(name) => {
                write!(f, "Collection '{name}' already exists.")
            }
            StoreError::AmbiguousId(id, matches) => {
                write!(f, "Task id '{id}' is ambiguous: {}.", matches.join(", "))
            }
            StoreError::DuplicateId(id) => write!(f, "Task id '{id}' already exists."),
            StoreError::InvalidNote => write!(f, "Note body cannot be empty."),
            StoreError::FileLockFailed(reason) => write!(f, "Could not lock task store: {reason}"),
            StoreError::Io(reason) => write!(f, "{reason}"),
            StoreError::Serde(reason) => write!(f, "{reason}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(value: std::io::Error) -> Self {
        StoreError::Io(value.to_string())
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(value: serde_json::Error) -> Self {
        StoreError::Serde(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, StoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_messages() {
        assert_eq!(
            StoreError::InvalidTitle.to_string(),
            "Task title cannot be empty."
        );
        assert_eq!(
            StoreError::NotFound("ab".into()).to_string(),
            "No task matches 'ab'."
        );
        assert_eq!(
            StoreError::AmbiguousId("a".into(), vec!["a1".into(), "a2".into()]).to_string(),
            "Task id 'a' is ambiguous: a1, a2."
        );
    }
}

//! Pond core: domain model and file-backed task store.

pub mod error;

pub use error::{Result, StoreError};

pub mod model;

pub use model::{
    CollectionColor, CollectionGroupSummary, CollectionSummary, TaskItem, TaskNote, TaskStatus,
};

pub mod ids;

pub mod json;

pub mod document;

pub use document::{TaskCollectionGroup, TaskFile};

pub mod paths;

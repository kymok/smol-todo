use crate::model::{CollectionColor, TaskItem};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCollectionGroup {
    pub name: String,
    pub collections: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFile {
    pub version: u32,
    #[serde(default)]
    pub collections: Vec<String>,
    #[serde(default)]
    pub collection_groups: Vec<TaskCollectionGroup>,
    #[serde(default)]
    pub collection_colors: BTreeMap<String, CollectionColor>,
    #[serde(default)]
    pub collection_prompts: BTreeMap<String, String>,
    #[serde(default)]
    pub archived_collections: BTreeSet<String>,
    #[serde(default)]
    pub items: Vec<TaskItem>,
}

impl Default for TaskFile {
    fn default() -> Self {
        TaskFile {
            version: 1,
            collections: Vec::new(),
            collection_groups: Vec::new(),
            collection_colors: BTreeMap::new(),
            collection_prompts: BTreeMap::new(),
            archived_collections: BTreeSet::new(),
            items: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::to_pretty_sorted;

    #[test]
    fn empty_document_round_trips() {
        let file = TaskFile::default();
        let json = to_pretty_sorted(&file).unwrap();
        let parsed: TaskFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, file);
        assert_eq!(parsed.version, 1);
    }

    #[test]
    fn camel_case_keys_used() {
        let json = to_pretty_sorted(&TaskFile::default()).unwrap();
        assert!(json.contains("collectionGroups"));
        assert!(json.contains("archivedCollections"));
    }

    #[test]
    fn missing_optional_sections_default() {
        let parsed: TaskFile = serde_json::from_str(r#"{"version":1}"#).unwrap();
        assert!(parsed.items.is_empty());
        assert!(parsed.collections.is_empty());
    }
}

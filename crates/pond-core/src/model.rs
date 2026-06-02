use crate::ids::make_version;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Draft,
    Ready,
    #[serde(rename = "in-progress")]
    InProgress,
    Completed,
    #[serde(rename = "on-hold")]
    OnHold,
    Rejected,
    Aborted,
}

impl TaskStatus {
    /// Declaration/UI order, matching Swift `TaskStatus.allCases`.
    pub fn all() -> [TaskStatus; 7] {
        [
            TaskStatus::Draft,
            TaskStatus::Ready,
            TaskStatus::InProgress,
            TaskStatus::Completed,
            TaskStatus::OnHold,
            TaskStatus::Rejected,
            TaskStatus::Aborted,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TaskStatus::Draft => "Draft",
            TaskStatus::Ready => "Ready",
            TaskStatus::InProgress => "In Progress",
            TaskStatus::Completed => "Completed",
            TaskStatus::OnHold => "On Hold",
            TaskStatus::Rejected => "Rejected",
            TaskStatus::Aborted => "Aborted",
        }
    }

    pub fn is_incomplete(&self) -> bool {
        *self != TaskStatus::Completed
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionColor {
    #[default]
    Gray,
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskNote {
    pub id: String,
    pub version: String,
    pub body: String,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskItem {
    pub id: String,
    pub version: String,
    pub title: String,
    pub collection: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<TaskNote>,
    pub status: TaskStatus,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

impl TaskItem {
    /// Build an item with a **provisional** random `version`. The persisted
    /// uniqueness guarantee lives in the store layer: when an item is saved, the
    /// store overwrites `version` with one generated against the set of all
    /// existing versions. `new` has no knowledge of existing versions, so this
    /// value is only a placeholder until the item is persisted.
    pub fn new(
        id: String,
        title: String,
        collection: String,
        status: TaskStatus,
        now: DateTime<Utc>,
    ) -> Self {
        TaskItem {
            id,
            version: make_version(&HashSet::new()),
            title,
            collection,
            note: None,
            status,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionSummary {
    pub name: String,
    pub display_name: String,
    pub group_name: String,
    pub total_count: usize,
    pub incomplete_count: usize,
    pub status_indicator: Option<TaskStatus>,
    pub color: CollectionColor,
    pub is_archived: bool,
    pub prompt_template: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionGroupSummary {
    pub name: String,
    pub collections: Vec<CollectionSummary>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn status_serializes_to_raw_values() {
        assert_eq!(
            serde_json::to_string(&TaskStatus::InProgress).unwrap(),
            "\"in-progress\""
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::OnHold).unwrap(),
            "\"on-hold\""
        );
        assert_eq!(
            serde_json::to_string(&TaskStatus::Ready).unwrap(),
            "\"ready\""
        );
        let parsed: TaskStatus = serde_json::from_str("\"in-progress\"").unwrap();
        assert_eq!(parsed, TaskStatus::InProgress);
    }

    #[test]
    fn incomplete_excludes_completed_only() {
        assert!(TaskStatus::Ready.is_incomplete());
        assert!(TaskStatus::Aborted.is_incomplete());
        assert!(!TaskStatus::Completed.is_incomplete());
    }

    #[test]
    fn color_round_trips() {
        assert_eq!(
            serde_json::to_string(&CollectionColor::Purple).unwrap(),
            "\"purple\""
        );
        let parsed: CollectionColor = serde_json::from_str("\"green\"").unwrap();
        assert_eq!(parsed, CollectionColor::Green);
        assert_eq!(CollectionColor::default(), CollectionColor::Gray);
    }

    #[test]
    fn item_serializes_note_as_singular_and_omits_when_absent() {
        let now = chrono::Utc.with_ymd_and_hms(2026, 6, 2, 12, 0, 0).unwrap();
        let mut item = TaskItem::new(
            "0123abcd".into(),
            "Buy milk".into(),
            "Inbox".into(),
            TaskStatus::Ready,
            now,
        );
        let json = serde_json::to_string(&item).unwrap();
        assert!(
            !json.contains("\"note\""),
            "absent note must be omitted: {json}"
        );

        item.note = Some(TaskNote {
            id: "ffff0000".into(),
            version: "abcdefabcdef".into(),
            body: "2%".into(),
            created_at: now,
            updated_at: now,
        });
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"note\""));
        let round: TaskItem = serde_json::from_str(&json).unwrap();
        assert_eq!(round, item);
    }

    #[test]
    fn summary_constructs() {
        let summary = CollectionSummary {
            name: "Work/Tasks".into(),
            display_name: "Tasks".into(),
            group_name: "Work".into(),
            total_count: 3,
            incomplete_count: 2,
            status_indicator: Some(TaskStatus::OnHold),
            color: CollectionColor::Blue,
            is_archived: false,
            prompt_template: None,
        };
        assert_eq!(summary.incomplete_count, 2);
        let group = CollectionGroupSummary {
            name: "Work".into(),
            collections: vec![summary],
        };
        assert_eq!(group.collections.len(), 1);
    }
}

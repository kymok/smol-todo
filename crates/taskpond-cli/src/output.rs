use pond_core::{CollectionColor, CollectionSummary, TaskItem, TaskStatus};
use serde::Serialize;

#[derive(Serialize)]
pub struct NoteOutput {
    pub id: String,
    pub version: String,
    pub body: String,
}

#[derive(Serialize)]
pub struct ItemOutput {
    pub id: String,
    pub status: TaskStatus, // serializes to its rawValue (e.g. "in-progress")
    pub collection: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<NoteOutput>,
}

impl ItemOutput {
    pub fn from_item(item: &TaskItem) -> Self {
        ItemOutput {
            id: item.id.clone(),
            status: item.status,
            collection: item.collection.clone(),
            title: item.title.clone(),
            note: item.note.as_ref().map(|n| NoteOutput {
                id: n.id.clone(),
                version: n.version.clone(),
                body: n.body.clone(),
            }),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionOutput {
    pub name: String,
    pub total_count: usize,
    pub incomplete_count: usize,
    pub color: CollectionColor,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_indicator: Option<TaskStatus>,
}

impl CollectionOutput {
    pub fn from_summary(summary: &CollectionSummary) -> Self {
        CollectionOutput {
            name: summary.name.clone(),
            total_count: summary.total_count,
            incomplete_count: summary.incomplete_count,
            color: summary.color,
            status_indicator: summary.status_indicator,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use pond_core::json::to_pretty_sorted;

    fn item() -> TaskItem {
        let now = Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap();
        let mut it = TaskItem::new(
            "0123abcd".into(),
            "Buy milk".into(),
            "Inbox".into(),
            TaskStatus::InProgress,
            now,
        );
        it.version = "v".repeat(12);
        it
    }

    #[test]
    fn item_output_omits_note_and_dates_uses_raw_status() {
        let json = to_pretty_sorted(&ItemOutput::from_item(&item())).unwrap();
        assert!(json.contains("\"status\": \"in-progress\""));
        assert!(!json.contains("note"));
        assert!(!json.contains("createdAt") && !json.contains("updatedAt"));
        // keys are sorted: collection before id before status before title
        let c = json.find("collection").unwrap();
        let t = json.find("title").unwrap();
        assert!(c < t);
    }

    #[test]
    fn collection_output_camel_case_and_raw_color() {
        let summary = CollectionSummary {
            name: "Work/A".into(),
            display_name: "A".into(),
            group_name: "Work".into(),
            total_count: 3,
            incomplete_count: 2,
            status_indicator: Some(TaskStatus::OnHold),
            color: CollectionColor::Blue,
            is_archived: false,
            prompt_template: None,
        };
        let json = to_pretty_sorted(&CollectionOutput::from_summary(&summary)).unwrap();
        assert!(json.contains("\"incompleteCount\": 2"));
        assert!(json.contains("\"color\": \"blue\""));
        assert!(json.contains("\"statusIndicator\": \"on-hold\""));
    }
}

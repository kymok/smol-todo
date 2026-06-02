use pond_core::{CollectionColor, CollectionGroupSummary, CollectionSummary, TaskItem, TaskStatus};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionSummaryDto {
    pub name: String,
    pub display_name: String,
    pub group_name: String,
    pub total_count: usize,
    pub incomplete_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_indicator: Option<TaskStatus>,
    pub color: CollectionColor,
    pub is_archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_template: Option<String>,
}

impl From<&CollectionSummary> for CollectionSummaryDto {
    fn from(s: &CollectionSummary) -> Self {
        CollectionSummaryDto {
            name: s.name.clone(),
            display_name: s.display_name.clone(),
            group_name: s.group_name.clone(),
            total_count: s.total_count,
            incomplete_count: s.incomplete_count,
            status_indicator: s.status_indicator,
            color: s.color,
            is_archived: s.is_archived,
            prompt_template: s.prompt_template.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionGroupSummaryDto {
    pub name: String,
    pub collections: Vec<CollectionSummaryDto>,
}

impl From<&CollectionGroupSummary> for CollectionGroupSummaryDto {
    fn from(g: &CollectionGroupSummary) -> Self {
        CollectionGroupSummaryDto {
            name: g.name.clone(),
            collections: g
                .collections
                .iter()
                .map(CollectionSummaryDto::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDto {
    pub items: Vec<TaskItem>,
    pub collections: Vec<CollectionSummaryDto>,
    pub groups: Vec<CollectionGroupSummaryDto>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collection_dto_is_camel_case_with_raw_values() {
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
        let json = serde_json::to_string(&CollectionSummaryDto::from(&summary)).unwrap();
        assert!(json.contains("\"displayName\":\"A\""));
        assert!(json.contains("\"groupName\":\"Work\""));
        assert!(json.contains("\"incompleteCount\":2"));
        assert!(json.contains("\"color\":\"blue\""));
        assert!(json.contains("\"statusIndicator\":\"on-hold\""));
        assert!(!json.contains("promptTemplate")); // None omitted
    }
}

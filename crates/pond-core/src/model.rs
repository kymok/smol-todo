use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}

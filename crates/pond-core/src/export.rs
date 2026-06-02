use crate::error::Result;
use crate::json::{to_compact_sorted, to_pretty_sorted};
use crate::model::TaskItem;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Jsonl,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPayload {
    pub collection: String,
    pub exported_at: DateTime<Utc>,
    pub items: Vec<TaskItem>,
}

impl ExportPayload {
    pub fn encode(&self, format: ExportFormat) -> Result<String> {
        match format {
            ExportFormat::Json => to_pretty_sorted(self),
            ExportFormat::Jsonl => {
                if self.items.is_empty() {
                    return Ok(String::new());
                }
                let mut lines = Vec::with_capacity(self.items.len());
                for item in &self.items {
                    lines.push(to_compact_sorted(item)?);
                }
                Ok(format!("{}\n", lines.join("\n")))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn payload() -> ExportPayload {
        let now = Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap();
        let mut item = TaskItem::new(
            "00000001".into(),
            "t".into(),
            "Inbox".into(),
            crate::model::TaskStatus::Ready,
            now,
        );
        item.version = "v".repeat(12);
        ExportPayload {
            collection: "Inbox".into(),
            exported_at: now,
            items: vec![item],
        }
    }

    #[test]
    fn json_is_pretty_with_wrapper() {
        let out = payload().encode(ExportFormat::Json).unwrap();
        assert!(out.contains("\"collection\""));
        assert!(out.contains("\"exportedAt\""));
        assert!(out.contains("\"items\""));
        assert!(out.starts_with("{\n"));
    }

    #[test]
    fn jsonl_is_one_item_per_line_trailing_newline() {
        let out = payload().encode(ExportFormat::Jsonl).unwrap();
        assert!(out.ends_with('\n'));
        assert_eq!(out.lines().count(), 1);
        assert!(
            !out.contains("exportedAt"),
            "jsonl emits raw items, not the wrapper"
        );
    }

    #[test]
    fn empty_jsonl_is_empty_string() {
        let mut p = payload();
        p.items.clear();
        assert_eq!(p.encode(ExportFormat::Jsonl).unwrap(), "");
    }
}

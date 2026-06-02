use pond_core::{CollectionColor, TaskStatus};

/// Port of Swift `cliUnescaped`: \n \r \t \\ become their control chars; an
/// unknown escape keeps the backslash; a trailing backslash is preserved.
pub fn unescape(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut escaping = false;
    for ch in input.chars() {
        if escaping {
            match ch {
                'n' => result.push('\n'),
                'r' => result.push('\r'),
                't' => result.push('\t'),
                '\\' => result.push('\\'),
                other => {
                    result.push('\\');
                    result.push(other);
                }
            }
            escaping = false;
        } else if ch == '\\' {
            escaping = true;
        } else {
            result.push(ch);
        }
    }
    if escaping {
        result.push('\\');
    }
    result
}

const STATUS_HELP: &str =
    "Expected 'ready', 'draft', 'in-progress', 'completed', 'on-hold', 'aborted', or 'rejected'.";
const COLOR_HELP: &str =
    "Expected 'gray', 'red', 'orange', 'yellow', 'green', 'blue', or 'purple'.";

pub fn parse_status(value: &str) -> Result<TaskStatus, String> {
    match value {
        "draft" => Ok(TaskStatus::Draft),
        "ready" => Ok(TaskStatus::Ready),
        "in-progress" => Ok(TaskStatus::InProgress),
        "completed" => Ok(TaskStatus::Completed),
        "on-hold" => Ok(TaskStatus::OnHold),
        "rejected" => Ok(TaskStatus::Rejected),
        "aborted" => Ok(TaskStatus::Aborted),
        _ => Err(STATUS_HELP.to_string()),
    }
}

pub fn parse_color(value: &str) -> Result<CollectionColor, String> {
    match value {
        "gray" => Ok(CollectionColor::Gray),
        "red" => Ok(CollectionColor::Red),
        "orange" => Ok(CollectionColor::Orange),
        "yellow" => Ok(CollectionColor::Yellow),
        "green" => Ok(CollectionColor::Green),
        "blue" => Ok(CollectionColor::Blue),
        "purple" => Ok(CollectionColor::Purple),
        _ => Err(COLOR_HELP.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescape_handles_known_and_unknown_escapes() {
        assert_eq!(unescape(r"a\nb"), "a\nb");
        assert_eq!(unescape(r"a\tb"), "a\tb");
        assert_eq!(unescape(r"a\\b"), r"a\b");
        assert_eq!(unescape(r"a\qb"), r"a\qb"); // unknown escape keeps the backslash
        assert_eq!(unescape(r"trailing\"), r"trailing\");
    }

    #[test]
    fn status_parsing() {
        assert_eq!(parse_status("in-progress").unwrap(), TaskStatus::InProgress);
        assert_eq!(parse_status("ready").unwrap(), TaskStatus::Ready);
        assert!(parse_status("nope")
            .unwrap_err()
            .contains("Expected 'ready'"));
    }

    #[test]
    fn color_parsing() {
        assert_eq!(parse_color("purple").unwrap(), CollectionColor::Purple);
        assert!(parse_color("teal").unwrap_err().contains("Expected 'gray'"));
    }
}

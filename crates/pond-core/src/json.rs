use crate::error::Result;
use serde::Serialize;

/// Pretty-printed JSON with keys sorted alphabetically.
pub fn to_pretty_sorted<T: Serialize>(value: &T) -> Result<String> {
    let value = serde_json::to_value(value)?;
    Ok(serde_json::to_string_pretty(&value)?)
}

/// Compact JSON with keys sorted alphabetically.
pub fn to_compact_sorted<T: Serialize>(value: &T) -> Result<String> {
    let value = serde_json::to_value(value)?;
    Ok(serde_json::to_string(&value)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Sample {
        zebra: i32,
        apple: i32,
    }

    #[test]
    fn keys_are_sorted() {
        let json = to_compact_sorted(&Sample { zebra: 1, apple: 2 }).unwrap();
        assert_eq!(json, r#"{"apple":2,"zebra":1}"#);
    }

    #[test]
    fn pretty_is_multiline_and_sorted() {
        let json = to_pretty_sorted(&Sample { zebra: 1, apple: 2 }).unwrap();
        assert!(json.starts_with("{\n"));
        let apple = json.find("apple").unwrap();
        let zebra = json.find("zebra").unwrap();
        assert!(apple < zebra);
    }
}

use crate::error::{Result, StoreError};
use std::collections::HashSet;

pub const DEFAULT_COLLECTION: &str = "Inbox";
pub const DEFAULT_GROUP: &str = "DefaultGroup";

#[derive(Debug)]
pub struct CollectionReference {
    pub group_name: String,
    pub display_name: String,
}

impl CollectionReference {
    pub fn api_name(&self) -> String {
        collection_api_name(&self.group_name, &self.display_name)
    }
}

pub fn collection_api_name(group_name: &str, display_name: &str) -> String {
    if group_name == DEFAULT_GROUP {
        display_name.to_string()
    } else {
        format!("{group_name}/{display_name}")
    }
}

pub fn collection_display_name(collection: &str) -> String {
    match parse_reference(collection, DEFAULT_GROUP) {
        Ok(reference) => reference.display_name,
        Err(_) => collection.to_string(),
    }
}

pub fn collection_group_name_for_api(collection: &str) -> String {
    parse_reference(collection, DEFAULT_GROUP)
        .map(|r| r.group_name)
        .unwrap_or_else(|_| DEFAULT_GROUP.to_string())
}

/// Parse `Name` or `Group/Name` into a reference. Empty parts are invalid.
pub fn parse_reference(collection: &str, default_group: &str) -> Result<CollectionReference> {
    let default_group = normalized_explicit_group(default_group)?;
    let clean = collection.trim();
    if clean.is_empty() {
        return Err(StoreError::InvalidCollection);
    }
    let parts: Vec<&str> = clean.splitn(2, '/').collect();
    match parts.as_slice() {
        [name] => Ok(CollectionReference {
            group_name: default_group,
            display_name: normalized_display_name(name)?,
        }),
        [group, name] => Ok(CollectionReference {
            group_name: normalized_explicit_group(group)?,
            display_name: normalized_display_name(name)?,
        }),
        _ => Err(StoreError::InvalidCollection),
    }
}

fn normalized_display_name(display: &str) -> Result<String> {
    let clean = display.trim();
    if clean.is_empty() || clean.contains('/') {
        return Err(StoreError::InvalidCollection);
    }
    Ok(clean.to_string())
}

/// Empty/whitespace → default collection (`Inbox`); otherwise normalized api name.
pub fn normalized_collection(collection: &str) -> Result<String> {
    if collection.trim().is_empty() {
        return Ok(DEFAULT_COLLECTION.to_string());
    }
    Ok(parse_reference(collection, DEFAULT_GROUP)?.api_name())
}

/// Like `normalized_collection`, but empty is an error (used where a collection is required).
pub fn normalized_explicit_collection(collection: &str) -> Result<String> {
    if collection.trim().is_empty() {
        return Err(StoreError::InvalidCollection);
    }
    Ok(parse_reference(collection, DEFAULT_GROUP)?.api_name())
}

pub fn normalized_explicit_group(group: &str) -> Result<String> {
    let clean = group.trim();
    if clean.is_empty() || clean.contains('/') {
        return Err(StoreError::InvalidCollectionGroup);
    }
    Ok(clean.to_string())
}

/// Deduplicate collection names, preserving first-seen order. (Sorting the default
/// collection first is handled separately by `sorted_collection_names`.)
pub fn normalized_collection_list<I: IntoIterator<Item = String>>(collections: I) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for collection in collections {
        // Unparseable names fall back to the default collection, matching the original app's behavior.
        let clean =
            normalized_collection(&collection).unwrap_or_else(|_| DEFAULT_COLLECTION.to_string());
        if seen.insert(clean.clone()) {
            result.push(clean);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_name_is_default_group() {
        let r = parse_reference("Errands", DEFAULT_GROUP).unwrap();
        assert_eq!(r.group_name, DEFAULT_GROUP);
        assert_eq!(r.display_name, "Errands");
        assert_eq!(r.api_name(), "Errands");
    }

    #[test]
    fn grouped_name_round_trips() {
        let r = parse_reference("Work/Tasks", DEFAULT_GROUP).unwrap();
        assert_eq!(r.group_name, "Work");
        assert_eq!(r.display_name, "Tasks");
        assert_eq!(r.api_name(), "Work/Tasks");
        assert_eq!(collection_display_name("Work/Tasks"), "Tasks");
        assert_eq!(collection_group_name_for_api("Work/Tasks"), "Work");
        assert_eq!(collection_group_name_for_api("Errands"), DEFAULT_GROUP);
    }

    #[test]
    fn empty_rules() {
        assert_eq!(normalized_collection("  ").unwrap(), "Inbox");
        assert_eq!(
            normalized_explicit_collection("  ").unwrap_err(),
            StoreError::InvalidCollection
        );
        // splitn(2, '/') keeps the remainder, so "a/b/c" → group "a", display "b/c";
        // a display name containing '/' is rejected.
        assert_eq!(
            parse_reference("a/b/c", DEFAULT_GROUP).unwrap_err(),
            StoreError::InvalidCollection
        );
    }

    #[test]
    fn list_dedups_and_normalizes() {
        let list =
            normalized_collection_list(vec!["Inbox".into(), "Inbox".into(), "Work/A".into()]);
        assert_eq!(list, vec!["Inbox".to_string(), "Work/A".to_string()]);
    }
}

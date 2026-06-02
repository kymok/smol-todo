use crate::document::{TaskCollectionGroup, TaskFile};
use crate::error::{Result, StoreError};
use crate::model::CollectionColor;
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

pub fn collection_exists(collection: &str, file: &TaskFile) -> bool {
    file.collections.iter().any(|c| c == collection)
        || file.items.iter().any(|i| i.collection == collection)
}

pub fn collection_group_containing(collection: &str, file: &TaskFile) -> Option<String> {
    normalized_collection_groups(&file.collection_groups, &all_collection_names(file))
        .into_iter()
        .find(|g| g.collections.iter().any(|c| c == collection))
        .map(|g| g.name)
}

pub(crate) fn all_collection_names(file: &TaskFile) -> Vec<String> {
    let mut names = file.collections.clone();
    names.extend(file.items.iter().map(|i| i.collection.clone()));
    normalized_collection_list(names)
}

/// Rebuild groups: dedup names, keep the default group first, assign every known
/// collection to exactly one group (its api-name group if otherwise unassigned).
pub fn normalized_collection_groups(
    groups: &[TaskCollectionGroup],
    collections: &[String],
) -> Vec<TaskCollectionGroup> {
    let names = normalized_collection_list(collections.to_vec());
    let name_set: HashSet<&String> = names.iter().collect();
    let mut seen_groups = HashSet::new();
    let mut assigned = HashSet::new();
    let mut result: Vec<TaskCollectionGroup> = Vec::new();

    for group in groups {
        let clean_name = group.name.trim();
        if clean_name.is_empty() || seen_groups.contains(clean_name) {
            continue;
        }
        // Re-canonicalize each listed collection to this group's API name, then keep
        // only the ones that are real and not already assigned. A collection whose API
        // name belongs to a different group is dropped here and re-homed below, keeping
        // group membership consistent with API names (matches the original app).
        let mapped: Vec<String> = group
            .collections
            .iter()
            .map(|c| collection_api_name(clean_name, &collection_display_name(c)))
            .collect();
        let clean_collections: Vec<String> = normalized_collection_list(mapped)
            .into_iter()
            .filter(|c| name_set.contains(c) && !assigned.contains(c))
            .collect();
        for c in &clean_collections {
            assigned.insert(c.clone());
        }
        seen_groups.insert(clean_name.to_string());
        result.push(TaskCollectionGroup {
            name: clean_name.to_string(),
            collections: clean_collections,
        });
    }

    if !seen_groups.contains(DEFAULT_GROUP) {
        result.insert(
            0,
            TaskCollectionGroup {
                name: DEFAULT_GROUP.to_string(),
                collections: Vec::new(),
            },
        );
        seen_groups.insert(DEFAULT_GROUP.to_string());
    }

    for collection in names.iter().filter(|c| !assigned.contains(*c)) {
        let group_name = collection_group_name_for_api(collection);
        if let Some(group) = result.iter_mut().find(|g| g.name == group_name) {
            group.collections.push(collection.clone());
            group.collections = normalized_collection_list(group.collections.clone());
        } else {
            result.push(TaskCollectionGroup {
                name: group_name,
                collections: vec![collection.clone()],
            });
        }
    }

    // Default group first, then the rest in encountered order.
    let mut ordered: Vec<TaskCollectionGroup> = result
        .iter()
        .filter(|g| g.name == DEFAULT_GROUP)
        .cloned()
        .collect();
    ordered.extend(result.into_iter().filter(|g| g.name != DEFAULT_GROUP));
    ordered
}

pub fn normalize_groups_in_file(file: &mut TaskFile) {
    let names = all_collection_names(file);
    file.collection_groups = normalized_collection_groups(&file.collection_groups, &names);
}

pub fn remove_collection_from_groups(collection: &str, file: &mut TaskFile) {
    for group in &mut file.collection_groups {
        group.collections.retain(|c| c != collection);
    }
}

pub fn add_collection_group_if_missing(group: &str, file: &mut TaskFile) {
    normalize_groups_in_file(file);
    if !file.collection_groups.iter().any(|g| g.name == group) {
        file.collection_groups.push(TaskCollectionGroup {
            name: group.to_string(),
            collections: Vec::new(),
        });
    }
}

pub fn move_collection_in_file(collection: &str, group: &str, file: &mut TaskFile) {
    add_collection_group_if_missing(group, file);
    remove_collection_from_groups(collection, file);
    if let Some(target) = file.collection_groups.iter_mut().find(|g| g.name == group) {
        target.collections.push(collection.to_string());
        target.collections = normalized_collection_list(target.collections.clone());
    }
    normalize_groups_in_file(file);
}

/// Ensure a collection exists (and is colored gray by default), placing it in `group`
/// when given or in its api-name group when newly added.
pub fn add_collection_if_missing(
    collection: &str,
    group: Option<&str>,
    file: &mut TaskFile,
) -> Result<()> {
    let clean = normalized_collection(collection)?;
    let resolved_group = match group {
        Some(g) => normalized_explicit_group(g)?,
        None => collection_group_name_for_api(&clean),
    };
    let already = collection_exists(&clean, file);
    file.collections = normalized_collection_list(
        file.collections
            .iter()
            .cloned()
            .chain([clean.clone()])
            .collect::<Vec<_>>(),
    );
    file.collection_colors
        .entry(clean.clone())
        .or_insert(CollectionColor::Gray);

    if group.is_some() || (!already && collection_group_containing(&clean, file).is_none()) {
        move_collection_in_file(&clean, &resolved_group, file);
    } else {
        normalize_groups_in_file(file);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{TaskCollectionGroup, TaskFile};

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

    #[test]
    fn default_group_is_always_present_and_first() {
        let groups = normalized_collection_groups(&[], &[]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, DEFAULT_GROUP);
    }

    #[test]
    fn unassigned_collections_land_in_their_group() {
        let groups = normalized_collection_groups(&[], &["Inbox".into(), "Work/A".into()]);
        let default = groups.iter().find(|g| g.name == DEFAULT_GROUP).unwrap();
        assert!(default.collections.contains(&"Inbox".to_string()));
        let work = groups.iter().find(|g| g.name == "Work").unwrap();
        assert_eq!(work.collections, vec!["Work/A".to_string()]);
    }

    #[test]
    fn add_collection_if_missing_colors_gray_and_groups() {
        let mut file = TaskFile::default();
        add_collection_if_missing("Work/A", None, &mut file).unwrap();
        assert!(collection_exists("Work/A", &file));
        assert_eq!(
            file.collection_colors.get("Work/A"),
            Some(&CollectionColor::Gray)
        );
        assert_eq!(
            collection_group_containing("Work/A", &file).as_deref(),
            Some("Work")
        );
    }

    #[test]
    fn group_membership_follows_api_name() {
        // A collection listed under a group that doesn't match its API name is re-homed
        // to the group its API name implies. "A" is a bare (default-group) API name, so
        // even when listed under "Work" it normalizes into DefaultGroup.
        let groups = vec![TaskCollectionGroup {
            name: "Work".into(),
            collections: vec!["A".into()],
        }];
        let normalized = normalized_collection_groups(&groups, &["A".into()]);
        let default = normalized.iter().find(|g| g.name == DEFAULT_GROUP).unwrap();
        assert!(default.collections.contains(&"A".to_string()));
        assert!(normalized
            .iter()
            .find(|g| g.name == "Work")
            .map_or(true, |g| !g.collections.contains(&"A".to_string())));
    }
}

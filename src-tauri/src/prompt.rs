use pond_core::prompt::APPLICATION_DEFAULT_TEMPLATE;

/// The effective app-default template: the stored default if it is non-empty
/// after trimming, else the built-in `APPLICATION_DEFAULT_TEMPLATE`.
/// Port of Swift `TaskPromptSettings.effectiveDefaultPromptTemplate`.
pub fn effective_default_template(stored_default: &str) -> &str {
    if stored_default.trim().is_empty() {
        APPLICATION_DEFAULT_TEMPLATE
    } else {
        stored_default
    }
}

/// The effective template for a collection: the collection's own override if it
/// is non-empty after trimming, else the effective app-default. Port of Swift
/// `CollectionMenus.effectivePromptTemplate`.
pub fn effective_collection_template(
    collection_template: Option<&str>,
    stored_default: &str,
) -> String {
    match collection_template {
        Some(t) if !t.trim().is_empty() => t.to_string(),
        _ => effective_default_template(stored_default).to_string(),
    }
}

/// The `taskpond` command that lists a collection's items. Port of Swift
/// `CollectionMenus.cliCommand`.
pub fn cli_command(name: &str) -> String {
    format!("taskpond item get --collection {}", shell_escape(name))
}

/// Single-quote a value for POSIX shells, escaping embedded single quotes as
/// `'\''`. EXACT port of Swift `String.shellEscaped`
/// (`"'\(replacingOccurrences(of: "'", with: "'\\''"))'"`).
pub fn shell_escape(name: &str) -> String {
    format!("'{}'", name.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pond_core::prompt::APPLICATION_DEFAULT_TEMPLATE;

    #[test]
    fn effective_default_uses_stored_when_present() {
        assert_eq!(effective_default_template("My default"), "My default");
    }

    #[test]
    fn effective_default_falls_back_when_empty_or_whitespace() {
        assert_eq!(effective_default_template(""), APPLICATION_DEFAULT_TEMPLATE);
        assert_eq!(
            effective_default_template("   \n\t "),
            APPLICATION_DEFAULT_TEMPLATE
        );
    }

    #[test]
    fn effective_collection_prefers_override() {
        assert_eq!(
            effective_collection_template(Some("Collection prompt"), "Stored default"),
            "Collection prompt"
        );
    }

    #[test]
    fn effective_collection_falls_back_to_stored_default() {
        // Override absent or whitespace-only → the stored default.
        assert_eq!(
            effective_collection_template(None, "Stored default"),
            "Stored default"
        );
        assert_eq!(
            effective_collection_template(Some("   "), "Stored default"),
            "Stored default"
        );
    }

    #[test]
    fn effective_collection_falls_back_to_builtin_when_both_empty() {
        assert_eq!(
            effective_collection_template(None, ""),
            APPLICATION_DEFAULT_TEMPLATE
        );
        assert_eq!(
            effective_collection_template(Some("  "), "  "),
            APPLICATION_DEFAULT_TEMPLATE
        );
    }

    #[test]
    fn shell_escape_plain_name() {
        assert_eq!(shell_escape("Work"), "'Work'");
    }

    #[test]
    fn shell_escape_name_with_spaces() {
        assert_eq!(shell_escape("Work Docs"), "'Work Docs'");
    }

    #[test]
    fn shell_escape_name_with_single_quote() {
        // A single quote becomes the 4-char sequence '\'' inside the wrapping quotes.
        assert_eq!(shell_escape("Bob's"), "'Bob'\\''s'");
    }

    #[test]
    fn cli_command_format() {
        assert_eq!(cli_command("Work"), "taskpond item get --collection 'Work'");
        assert_eq!(
            cli_command("Bob's list"),
            "taskpond item get --collection 'Bob'\\''s list'"
        );
    }
}

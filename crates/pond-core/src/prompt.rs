use std::collections::HashMap;

pub const APPLICATION_DEFAULT_TEMPLATE: &str = "Run `{{cliCommand}}` and complete the listed tasks. Use `taskpond item update [task id] --status [status]` to update task status. Skip `Draft` tasks. Mark unclear, unnatural, or clearly unrelated tasks as `on-hold`. Mark tasks as `in-progress` when started and `aborted` if they cannot be completed. Group related work into appropriate commits. Use sub-agents with separate worktrees when parallelization helps, then merge their branches into the current branch. Before finishing, run `{{cliCommand}}` again because the user may add more tasks, and ensure no uncommitted changes remain.";

/// Substitute `{{token}}` occurrences from `variables`; unknown tokens are kept verbatim.
pub fn evaluate(template: &str, variables: &HashMap<String, String>) -> String {
    let mut result = String::new();
    let mut rest = template;
    while let Some(open) = rest.find("{{") {
        result.push_str(&rest[..open]);
        let after_open = &rest[open + 2..];
        match after_open.find("}}") {
            Some(close) => {
                let token = after_open[..close].trim();
                match variables.get(token) {
                    Some(value) => result.push_str(value),
                    None => {
                        result.push_str("{{");
                        result.push_str(&after_open[..close]);
                        result.push_str("}}");
                    }
                }
                rest = &after_open[close + 2..];
            }
            None => {
                result.push_str(&rest[open..]);
                return result;
            }
        }
    }
    result.push_str(rest);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_known_tokens() {
        let vars: HashMap<String, String> = [(
            "cliCommand".to_string(),
            "taskpond item get -c X".to_string(),
        )]
        .into_iter()
        .collect();
        let out = evaluate("Run `{{cliCommand}}` now", &vars);
        assert_eq!(out, "Run `taskpond item get -c X` now");
    }

    #[test]
    fn keeps_unknown_tokens_verbatim() {
        let out = evaluate("a {{missing}} b", &HashMap::new());
        assert_eq!(out, "a {{missing}} b");
    }

    #[test]
    fn default_template_mentions_cli_command() {
        assert!(APPLICATION_DEFAULT_TEMPLATE.contains("{{cliCommand}}"));
    }
}

use ruxex_extension_api::ToolName;
use ruxex_extension_api::ToolPolicy;

#[test]
fn default_policy_allows_all_tools() {
    let policy = ToolPolicy::default();

    assert!(policy.allows(&ToolName::plain("exec_command")));
    assert!(policy.allows(&ToolName::namespaced("mcp", "lookup")));
}

#[test]
fn allowlist_policy_matches_namespaces_and_empty_lists() {
    let policy = ToolPolicy {
        allowed_tools: Some(vec![ToolName::plain("exec_command")]),
    };
    assert!(policy.allows(&ToolName::plain("exec_command")));
    assert!(!policy.allows(&ToolName::namespaced("mcp", "exec_command")));
    assert!(!policy.allows(&ToolName::plain("view_image")));

    let empty = ToolPolicy {
        allowed_tools: Some(Vec::new()),
    };
    assert!(!empty.allows(&ToolName::plain("exec_command")));
}

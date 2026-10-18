//! Environment a panel's process must not inherit: the per-session markers a parent agent
//! CLI session sets for its own children (AgentFlow may itself run inside one).
/// Per-session variables a parent Claude Code session sets for its own children. Settings
/// such as `CLAUDE_CONFIG_DIR` are kept.
pub const PARENT_SESSION_VARS: [&str; 10] = [
    "CLAUDECODE",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_SESSION_ATTENDED",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_EXECPATH",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CLAUDE_CODE_SSE_PORT",
    "CLAUDE_PID",
];

#[cfg(test)]
mod tests {
    use super::PARENT_SESSION_VARS;

    #[test]
    fn parent_agent_session_markers_are_not_inherited() {
        assert!(PARENT_SESSION_VARS.contains(&"CLAUDE_CODE_CHILD_SESSION"));
        assert!(PARENT_SESSION_VARS.contains(&"CLAUDECODE"));
        assert!(!PARENT_SESSION_VARS.contains(&"CLAUDE_CONFIG_DIR"));
    }
}

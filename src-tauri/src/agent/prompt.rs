//! AgentFlow's default system prompt. The environment part (OS, shell, workspace, a capped
//! file tree, the project's AGENTS.md / CLAUDE.md) is built once per conversation and stored
//! with it, so the prompt stays byte-identical and keeps the cached prefix (and Anthropic's
//! thinking) valid; the mode and tool lines follow the current request.

use std::fmt::Write as _;
use std::path::Path;

use crate::conversations::PermissionMode;
use crate::fs::walker;
use crate::providers::ToolSpec;

/// Entries of the top-level tree in the prompt.
pub const MAX_TREE_ENTRIES: usize = 120;
/// Project instructions are cut here.
pub const MAX_INSTRUCTIONS_BYTES: usize = 8 * 1024;
const INSTRUCTION_FILES: [&str; 2] = ["AGENTS.md", "CLAUDE.md"];

/// The environment part: built once per conversation.
pub fn environment(workspace: Option<&Path>, shell: &str) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "## Environment");
    let _ = writeln!(out, "- OS: {}", std::env::consts::OS);
    let _ = writeln!(out, "- Shell for run_command: {shell}");
    let Some(root) = workspace else {
        let _ = writeln!(
            out,
            "- No workspace folder is open: file tools are unavailable."
        );
        return out;
    };
    // Without Windows' verbatim prefix (`\\?\C:\...`): models copy the path into commands.
    let shown = root.display().to_string();
    let shown = shown.strip_prefix(r"\\?\").unwrap_or(&shown);
    let _ = writeln!(out, "- Workspace: {shown}");
    let _ = writeln!(
        out,
        "\n## Workspace files (top levels, .gitignore respected)"
    );
    out.push_str(&tree(root));
    if let Some((name, text)) = instructions(root) {
        let _ = writeln!(out, "\n## Project instructions ({name})");
        out.push_str(&text);
        out.push('\n');
    }
    out
}

/// The top two levels, folders with a trailing `/`, capped.
pub fn tree(root: &Path) -> String {
    let mut entries: Vec<String> = Vec::new();
    let mut more = 0usize;
    for entry in walker(root, false).max_depth(Some(2)).build().flatten() {
        if entry.depth() == 0 {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        if entries.len() >= MAX_TREE_ENTRIES {
            more += 1;
            continue;
        }
        let rel = rel.to_string_lossy().replace('\\', "/");
        let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
        entries.push(if is_dir { format!("{rel}/") } else { rel });
    }
    entries.sort();
    let mut out = entries.join("\n");
    if more > 0 {
        let _ = write!(out, "\n… and {more} more (use list_dir or glob)");
    }
    if out.is_empty() {
        out.push_str("(empty)");
    }
    out.push('\n');
    out
}

/// AGENTS.md, else CLAUDE.md, at the workspace root, cut at `MAX_INSTRUCTIONS_BYTES`.
pub fn instructions(root: &Path) -> Option<(&'static str, String)> {
    INSTRUCTION_FILES.iter().find_map(|name| {
        let text = std::fs::read_to_string(root.join(name)).ok()?;
        let text = text.trim();
        (!text.is_empty()).then(|| (*name, super::tools::truncate(text, MAX_INSTRUCTIONS_BYTES)))
    })
}

const RULES: &str = "## Rules
- Read a file before you edit it; prefer edit_file (exact, unique old_string) over rewriting \
whole files with write_file.
- Use paths relative to the workspace. Stay inside it.
- Before a destructive step (deleting, overwriting, force operations), say what you are about \
to do and why.
- If the user denies a tool call, do not retry it; follow their reason or ask.
- Keep answers short; show code or file names rather than describing them at length.";

/// The full system prompt: identity, environment, tools and mode, rules, then the user's
/// own prompt. `default_prompt == false` sends the user's prompt only.
pub fn system_prompt(
    environment: &str,
    tools: &[ToolSpec],
    mode: PermissionMode,
    user_prompt: &str,
    default_prompt: bool,
) -> String {
    let user_prompt = user_prompt.trim();
    if !default_prompt {
        return user_prompt.to_owned();
    }
    let mut out = String::from(
        "You are an AI agent running inside AgentFlow, a desktop app for coding with AI \
         agents. You work in the user's workspace through tools.\n\n",
    );
    out.push_str(environment.trim_end());
    out.push_str("\n\n## Tools\n");
    if tools.is_empty() {
        out.push_str("No tools in this chat: answer from the conversation only.\n");
    } else {
        for tool in tools {
            let _ = writeln!(out, "- {}: {}", tool.name, tool.description);
        }
        let mode_line = match mode {
            PermissionMode::Ask => {
                "Ask: reads run directly; writes and commands need the user's approval."
            }
            PermissionMode::AutoEdit => {
                "Auto-edit: reads and file changes run directly; commands need approval."
            }
            PermissionMode::ReadOnly => {
                "Read-only: you can read and search, not change files or run commands."
            }
        };
        let _ = writeln!(out, "\nPermission mode: {mode_line}");
    }
    out.push('\n');
    out.push_str(RULES);
    if !user_prompt.is_empty() {
        out.push_str("\n\n## The user's instructions\n");
        out.push_str(user_prompt);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_dir::TestDir;

    #[test]
    fn caps_the_tree_and_the_project_instructions() {
        let dir = TestDir::new("prompt");
        for i in 0..(MAX_TREE_ENTRIES + 30) {
            std::fs::write(dir.path().join(format!("f{i:03}.txt")), "x").expect("write");
        }
        std::fs::write(
            dir.path().join("AGENTS.md"),
            "a".repeat(MAX_INSTRUCTIONS_BYTES * 2),
        )
        .expect("write");
        let tree = tree(dir.path());
        assert_eq!(tree.lines().count(), MAX_TREE_ENTRIES + 1);
        assert!(tree.contains("… and 31 more"), "{tree}");
        let (name, text) = instructions(dir.path()).expect("instructions");
        assert_eq!(name, "AGENTS.md");
        assert!(text.len() < MAX_INSTRUCTIONS_BYTES + 64);
        assert!(text.ends_with(&format!("[truncated, {MAX_INSTRUCTIONS_BYTES} more bytes]")));
    }

    #[test]
    fn claude_md_is_the_fallback_and_the_prompt_has_all_parts() {
        let dir = TestDir::new("prompt-claude");
        std::fs::write(dir.path().join("CLAUDE.md"), "Use tabs.").expect("write");
        let env = environment(Some(dir.path()), "pwsh");
        assert!(env.contains("Shell for run_command: pwsh"));
        assert!(env.contains("## Project instructions (CLAUDE.md)\nUse tabs."));
        let tools = crate::agent::tools::specs(PermissionMode::ReadOnly);
        let prompt = system_prompt(&env, &tools, PermissionMode::ReadOnly, "Be brief.", true);
        assert!(prompt.starts_with("You are an AI agent running inside AgentFlow"));
        assert!(prompt.contains("- read_file:") && !prompt.contains("- write_file:"));
        assert!(prompt.contains("Permission mode: Read-only"));
        assert!(prompt.ends_with("## The user's instructions\nBe brief."));
        assert_eq!(
            system_prompt(&env, &tools, PermissionMode::Ask, " Be brief. ", false),
            "Be brief."
        );
    }
}

# Agent targets

Where the agent installer writes, and why. Checked against the official docs on
**2026-10-17**; re-check when a tool changes its format. Project scope = inside the workspace;
user scope = the home folder, only where the tool documents it, after a separate confirmation.

| Target      | Agent file (project)        | Shared instructions (managed section) | User scope                                 |
| ----------- | --------------------------- | ------------------------------------- | ------------------------------------------ |
| Claude Code | `.claude/agents/<id>.md`    | `CLAUDE.md`                           | `~/.claude/agents/`, `~/.claude/CLAUDE.md` |
| Codex CLI   | `.codex/agents/<id>.toml`   | `AGENTS.md`                           | `~/.codex/agents/`, `~/.codex/AGENTS.md`   |
| Gemini CLI  | `.gemini/agents/<id>.md`    | `GEMINI.md`                           | `~/.gemini/agents/`, `~/.gemini/GEMINI.md` |
| Cursor      | `.cursor/rules/<id>.mdc`    | —                                     | not file-based (in-app user rules)         |
| AgentFlow   | `.agentflow/agents/<id>.md` | —                                     | project only                               |

The agent file holds the role; the shared instruction file only gets a short managed section
that points to it (`<!-- agentflow:begin <id> -->` … `<!-- agentflow:end <id> -->`), so the
role does not apply to every session.

## Claude Code

- Subagents: Markdown with YAML frontmatter; the body is the subagent's system prompt.
  Required `name` and `description`; optional `tools` (comma-separated or a list; omitted =
  all tools), `model` (`sonnet`, `opus`, `haiku`, `inherit` or a model id), and more
  (`permissionMode`, `skills`, `color`, ...; unknown fields are ignored). Project
  `.claude/agents/`, user `~/.claude/agents/`; identity comes from `name`.
  Used automatically by description, by name in a prompt, or as `@agent-<name>`.
  <https://code.claude.com/docs/en/sub-agents>
- Skills (`.claude/skills/<name>/SKILL.md`, user `~/.claude/skills/`) exist too; the installer
  uses subagents, which carry a role. <https://code.claude.com/docs/en/skills>,
  naming per the Agent Skills spec <https://agentskills.io/specification>
- Instructions: `./CLAUDE.md` (or `./.claude/CLAUDE.md`), user `~/.claude/CLAUDE.md`, loaded
  from the working directory upward. <https://code.claude.com/docs/en/memory>

## Codex CLI

- `AGENTS.md`: read from the Git root down to the working directory (`AGENTS.override.md`
  first), global `~/.codex/AGENTS.md` (or `$CODEX_HOME`); 32 KiB combined by default.
  <https://learn.chatgpt.com/docs/agent-configuration/agents-md>
- Subagents: **TOML**, one per file, project `.codex/agents/`, user `~/.codex/agents/`.
  Required `name`, `description`, `developer_instructions`; optional `model` and reasoning
  options. Invoked by asking for the agent by name.
  <https://learn.chatgpt.com/docs/agent-configuration/subagents>
- Custom prompts (`~/.codex/prompts/*.md`) are deprecated in favour of skills and have no
  project scope: not used. <https://learn.chatgpt.com/docs/custom-prompts>
- Skills: `.agents/skills/` (repo) and `~/.agents/skills/`. <https://learn.chatgpt.com/docs/build-skills>

## Gemini CLI

- `GEMINI.md`: global `~/.gemini/GEMINI.md`, the workspace and its parents, plus files found
  near paths tools touch; the name is configurable (`context.fileName`).
  <https://geminicli.com/docs/cli/gemini-md/>
- Subagents (on by default): Markdown with YAML frontmatter, project `.gemini/agents/*.md`,
  user `~/.gemini/agents/*.md`. Required `name` (lowercase letters, digits, `-`, `_`) and
  `description`; optional `tools`, `model` (default `inherit`), `temperature`, `max_turns`,
  `timeout_mins`. Used automatically or with `@name`; `/agents` manages them.
  <https://geminicli.com/docs/core/subagents/>
- Custom commands (`.gemini/commands/*.toml`, `prompt` + `description`, `{{args}}`) and skills
  exist too. <https://geminicli.com/docs/cli/custom-commands/>,
  <https://geminicli.com/docs/cli/skills/>

## Cursor

- Project rules: `.cursor/rules/*.mdc` (plain `.md` is ignored there), frontmatter
  `description`, `globs` (comma-separated) and `alwaysApply`. Rule types: always
  (`alwaysApply: true`), by description (no globs), for matching files (globs), manual
  (`@rule-name`). Cursor also reads `AGENTS.md`. User rules live in the app only.
  <https://cursor.com/docs/context/rules>

## AgentFlow

- `.agentflow/agents/<id>.md`: YAML frontmatter (`name`, `description`, `provider`, `model`,
  `permissionMode`, `tools`, `character`) and the instructions as the body. See
  [agent-template.md](agent-template.md).

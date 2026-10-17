# Turn signals

How AgentFlow tells that an agent finished a turn, what it said last and what it is doing
right now. Checked against the official docs and source on **2026-10-18** (Claude Code
2.1.30x, Codex 0.16x, Gemini CLI main); re-check when a tool changes its log format. All
readers are read-only and reuse the phase 6 log readers (`src-tauri/src/tokens/cli/`):
the same roots, folder watcher and panel matching (same CLI, same working directory, inside
the process time window).

| Panel       | Turn end                                       | Report                       | Live activity           | Source     |
| ----------- | ---------------------------------------------- | ---------------------------- | ----------------------- | ---------- |
| API agent   | the chat stream's `stop` event                 | the final assistant message  | `status` / `toolStart`  | `stop`     |
| Claude Code | `assistant` with `stop_reason:"end_turn"`      | that message's `text` blocks | `tool_use` blocks       | `log`      |
| Codex CLI   | `event_msg` `task_complete`                    | `last_agent_message`         | `function_call`, ...    | `log`      |
| Gemini CLI  | none in the log: terminal idle                 | the last `gemini` message    | `toolCalls`             | `idle` (≈) |
| custom CLI  | terminal idle (2 s quiet after output)         | the terminal text since then | "Working…"              | `idle` (≈) |

`idle` results are marked **approximate** in the UI. Hooks are not used (see below).

## Claude Code

Session JSONL: `~/.claude/projects/<encoded cwd>/<session>.jsonl` (`$CLAUDE_CONFIG_DIR`).
<https://code.claude.com/docs/en/hooks> (fields), local files for the record shapes (the
transcript format itself is not documented).

- `assistant` records: one line per content block, all lines of a response share
  `message.id` and carry its final `stop_reason` (`tool_use` or `end_turn`). Blocks:
  `thinking` (activity: think), `text`, `tool_use {id, name, input}`.
- Turn end: an `assistant` line with `stop_reason:"end_turn"` (also `max_tokens`,
  `stop_sequence`), followed by a `system` record `subtype:"turn_duration"`. The report is
  the `text` blocks of that `message.id`.
- Tool results come back as `user` records with `tool_result {tool_use_id, is_error}`.
- Interrupt: a `user` record whose text is `[Request interrupted by user]` or
  `[Request interrupted by user for tool use]`: the turn ends without a report.
- Skipped: `isSidechain:true` records and the subagent files (`<session>/subagents/`),
  `isMeta` user records, `attachment`, `system` other than the above.
- Tool inputs used for activity: `Bash{command}`, `Read{file_path}`, `Edit|Write|
  MultiEdit{file_path}`, `NotebookEdit{notebook_path}`, `Grep|Glob{pattern}`,
  `WebFetch{url}`, `WebSearch{query}`, `Task|Agent{description}`.
- A pending permission prompt is not logged. A `tool_use` without its result while the
  terminal shows no output for 1.5 s is shown as "waiting for your approval" (approximate;
  the TUI animates while a tool runs, and holds still while it asks).

## Codex CLI

Rollouts: `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` (`$CODEX_HOME`), lines
`{timestamp, type, payload}`. Source:
<https://github.com/openai/codex/blob/main/codex-rs/protocol/src/protocol.rs>,
<https://github.com/openai/codex/blob/main/codex-rs/protocol/src/models.rs>,
<https://github.com/openai/codex/blob/main/codex-rs/rollout/src/policy.rs>.

- `event_msg` `task_started` (alias `turn_started`): a turn begins.
- `event_msg` `task_complete` (alias `turn_complete`): the turn ended;
  `last_agent_message` is the report.
- `event_msg` `turn_aborted {reason}` (`interrupted`, `replaced`, ...): ended, no report.
- `response_item` `reasoning`: think. `function_call {name, arguments}` (a JSON string:
  `shell_command{command}`, `exec_command{cmd}`, `shell{command[]}`, `update_plan`),
  `custom_tool_call {name, input}` (`apply_patch` = the files in `*** Update File:` /
  `*** Add File:` lines; `exec` = code that orchestrates tools), `local_shell_call
  {action.command}`: tool activity. `*_output` records: the call finished.
- `event_msg` `exec_approval_request` / `apply_patch_approval_request`: waiting for
  approval (written only by some versions).
- `ExecCommandBegin/End` are never persisted, so activity comes from the response items.
- `notify` (`config.toml`, event `agent-turn-complete` with `last-assistant-message`) is
  marked legacy in the source and would need a config edit: not used.
  <https://learn.chatgpt.com/docs/config-file/config-advanced>

## Gemini CLI

Chats: `~/.gemini/tmp/<project>/chats/session-*.jsonl` (`$GEMINI_CLI_HOME`). Source:
<https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/services/chatRecordingService.ts>.

- Message records `{id, timestamp, type: user|gemini|info|error|warning, content}`;
  `gemini` messages add `toolCalls [{id, name, args, status}]`, `thoughts`, `tokens`. The
  same id is appended again as it is updated (`$set` / `$patch` records also exist).
- There is no turn-end record. AgentFlow uses the terminal idle heuristic for the end and
  the last `gemini` message's `content` as the report (marked approximate).
- `user` record: a turn begins; `toolCalls` give activity; `error` records: error.
- Gemini CLI was replaced by Antigravity CLI for unpaid users on 2026-06-18
  (<https://geminicli.com/docs/hooks/>); its format is not covered.

## Hooks (not used)

All three tools have a turn-end hook with the final text: Claude Code `Stop`
(`last_assistant_message`), Codex `Stop` (same field), Gemini `AfterAgent`
(`prompt_response`), plus tool hooks and Claude's `Notification` (`permission_prompt`).
<https://code.claude.com/docs/en/hooks>, <https://learn.chatgpt.com/docs/hooks>,
<https://geminicli.com/docs/hooks/reference/>.

They are not installed: tailing the logs is exact for Claude Code and Codex without touching
the user's tool settings, Codex skips new project hooks until the user trusts them in
`/hooks`, and a hook command would have to reach the running app. Only Gemini would gain an
exact turn end; that is not worth a settings edit today. AgentFlow never edits tool
settings silently; if hooks are added later they go through the installer's preview and
confirmation, off by default.

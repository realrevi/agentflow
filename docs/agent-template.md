# Agent template format (schema 1)

One template describes an agent once; AgentFlow's installer renders it into the files of
each tool (see [agent-targets.md](agent-targets.md)). Built-in starters live in
`src-tauri/src/agents/starters/`; the community library (phase 9) will use the same format.
The JSON Schema is [`schemas/agent-template.schema.json`](../schemas/agent-template.schema.json).

```json
{
  "schema": 1,
  "id": "code-reviewer",
  "name": "Code Reviewer",
  "description": "Reviews changed code for bugs, risks and clarity. Use after changing code.",
  "version": "1.0.0",
  "author": "AgentFlow",
  "license": "MIT",
  "tags": ["review"],
  "character": "owl",
  "language": "en",
  "instructions": "You review {{workspaceName}}. Focus on: {{focus}}.",
  "questions": [
    {
      "id": "focus",
      "label": "What should the review focus on?",
      "type": "multiChoice",
      "options": ["correctness", "security", "tests"],
      "default": ["correctness"]
    }
  ],
  "targets": {
    "claudeCode": { "tools": ["Read", "Grep", "Glob", "Bash"], "model": "inherit" },
    "cursor": { "globs": ["**/*.ts"], "alwaysApply": false },
    "agentflow": { "permissionMode": "readOnly", "tools": ["read_file", "search"] }
  }
}
```

## Fields

| Field                       | Rule                                                                                   |
| --------------------------- | -------------------------------------------------------------------------------------- |
| `schema`                    | `1`                                                                                    |
| `id`                        | 2-48 of `a-z`, `0-9`, `-` (starts with a letter or digit); the file name in every tool |
| `name`                      | 1-80 characters                                                                        |
| `description`               | one line, 1-500 characters; tools use it to decide when to use the agent               |
| `version`                   | `major.minor.patch`; a higher version offers an update to installed copies             |
| `author`, `license`, `tags` | optional; at most 10 tags of `a-z0-9-`                                                 |
| `character`                 | optional suggested pixel character for AgentFlow (`owl`, `fox`, ...)                   |
| `language`                  | the language the instructions are written in (`en`, `pt-BR`, ...)                      |
| `instructions`              | the role, Markdown, up to 20 000 characters, with `{{placeholders}}`                   |
| `questions`                 | up to 30, see below                                                                    |
| `targets`                   | optional per-tool options, see below                                                   |

## Questions

`id` (a letter, then letters, digits or `_`), `label`, optional `help`, `type`, optional
`default`, `options` (for `choice` / `multiChoice`), `detect`, `required`, `group`.

| `type`                               | Answer       | In text      |
| ------------------------------------ | ------------ | ------------ |
| `text`, `multiline`, `color`, `font` | string       | as written   |
| `choice`                             | one option   | the option   |
| `multiChoice`                        | options      | `a, b, c`    |
| `boolean`                            | `true/false` | `yes` / `no` |

`detect` prefills the answer from the workspace (local and read-only; the user can change
it): `languages`, `packageManager`, `testFramework`, `testCommand`, `uiFramework`, `colors`,
`fonts`, `spacing`, `breakpoints`.

## Placeholders

`{{id}}` inserts the answer to question `id`; built-ins are `{{name}}`, `{{description}}` and
`{{workspaceName}}`. Rendering is plain substitution, nothing is evaluated. A placeholder
that names no question is a validation error; a `required` question without an answer is an
error at install. A line whose placeholders all come out empty is dropped, so optional
answers leave no stubs. Templates and answers may not contain `<!-- agentflow:` (the
managed-section marker).

## Targets

| Key          | Options                                                                        |
| ------------ | ------------------------------------------------------------------------------ |
| `claudeCode` | `tools` (Claude Code tool names; none = all), `model` (`inherit`, ...)         |
| `codex`      | none yet                                                                       |
| `gemini`     | none yet                                                                       |
| `cursor`     | `globs` (files the rule attaches to), `alwaysApply`                            |
| `agentflow`  | `permissionMode` (`ask`, `autoEdit`, `readOnly`), `tools`, `provider`, `model` |

An AgentFlow profile's `permissionMode` is never looser than the user's default, and its
`tools` only narrow what that mode allows.

## What gets written

Each target gets its own agent file with the rendered instructions; `CLAUDE.md`, `AGENTS.md`
and `GEMINI.md` only get a short managed section that points to it:

```markdown
<!-- agentflow:begin code-reviewer -->

## Code Reviewer

Reviews changed code ... Delegate matching tasks to the `code-reviewer` subagent.
<!-- agentflow:end code-reviewer -->
```

Everything outside the markers is left as it is. The installer records each file and a hash
in `.agentflow/installed.json`, so updates and uninstalls never overwrite or delete a file
the user changed without asking.

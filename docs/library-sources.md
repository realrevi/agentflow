# Agent library sources

The curated ("verified") sources of the community agent library, listed in
[`registry/sources.json`](../registry/sources.json). The app reads the newest copy of that file
from this repository's `main` branch on GitHub (falling back to the copy bundled with the
app), so sources can change without a release. Checked on **2026-10-18** with the GitHub API
(stars, last push, license, file tree).

Criteria: many stars, active (pushed within about six months), a clear folder structure, and
an OSI license that allows redistribution and modification. Repositories without a license
are skipped.

| Source                                                                                                | License                     | Contents                                                                            | Why                                                       |
| ----------------------------------------------------------------------------------------------------- | --------------------------- | ----------------------------------------------------------------------------------- | --------------------------------------------------------- |
| [anthropics/skills](https://github.com/anthropics/skills)                                             | per skill (Apache-2.0)      | ~15 Agent Skills (`skills/*/SKILL.md` + files)                                      | Anthropic's official skills; the reference format         |
| [openai/skills](https://github.com/openai/skills)                                                     | per skill (Apache-2.0, MIT) | ~27 curated Codex skills (`skills/.curated/*/`)                                     | OpenAI's official skills, same open format                |
| [obra/superpowers](https://github.com/obra/superpowers)                                               | MIT                         | 15 skills (`skills/*/`)                                                             | the most starred skills repository, small and clean       |
| [wshobson/agents](https://github.com/wshobson/agents)                                                 | MIT                         | ~200 Claude Code subagents and ~185 skills (`plugins/*/agents`, `plugins/*/skills`) | the most used Claude Code plugin marketplace, very active |
| [VoltAgent/awesome-claude-code-subagents](https://github.com/VoltAgent/awesome-claude-code-subagents) | MIT                         | 163 subagents (`categories/*/*.md`)                                                 | the canonical subagent collection                         |
| [VoltAgent/awesome-codex-subagents](https://github.com/VoltAgent/awesome-codex-subagents)             | MIT                         | 175 Codex agents (`categories/*/*.toml`)                                            | the only sizeable Codex-native collection                 |
| [PatrickJS/awesome-cursorrules](https://github.com/PatrickJS/awesome-cursorrules)                     | CC0-1.0                     | 257 Cursor rules (`rules/*.mdc`)                                                    | the largest Cursor rules collection                       |
| [sanjeed5/awesome-cursor-rules-mdc](https://github.com/sanjeed5/awesome-cursor-rules-mdc)             | CC0-1.0                     | 241 Cursor rules, one per library (`rules-mdc/*.mdc`)                               | clean, one rule per framework                             |
| [google-gemini/gemini-cli](https://github.com/google-gemini/gemini-cli)                               | Apache-2.0                  | Gemini custom commands and skills (`.gemini/commands`, `.gemini/skills`)            | an official Gemini source (see below)                     |

Per-skill licenses: in `anthropics/skills` and `openai/skills` each skill carries its own
`LICENSE.txt`. The importer reads it and only keeps skills under an OSI license (Apache-2.0,
MIT, BSD, ...). The proprietary `docx`, `pdf`, `pptx`, `xlsx` skills, `doc-coauthoring` (no
license), and OpenAI's `figma*` and `notion-*` skills (other terms) are also excluded in the
registry.

## Skipped

| Repository                                                          | Why                                                                      |
| ------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| anthropics/claude-code                                              | no license; the product repository, not a library                        |
| ComposioHQ/awesome-claude-skills                                    | no license; mostly generated items                                       |
| vercel-labs/agent-skills, mrgoonie/claudekit-skills                 | no license                                                               |
| contains-studio/agents, amitkmaraj/gemini-cli-custom-slash-commands | no license, inactive                                                     |
| hesreallyhim/awesome-claude-code, VoltAgent/awesome-agent-skills    | link lists, not item files                                               |
| palladius/gemini-cli-custom-commands                                | no clear license (NOASSERTION), small                                    |
| trailofbits/skills                                                  | CC-BY-SA-4.0: share-alike would bind changed copies                      |
| github/awesome-copilot                                              | MIT, but very large (435 skills, 14 MB) and Copilot-centred; maybe later |
| K-Dense-AI/scientific-agent-skills, alirezarezvani/claude-skills    | MIT, but huge or noisy (mirrors, thousands of files)                     |
| microsoft/skills, hashicorp/agent-skills, supabase/agent-skills     | niche (Azure, Terraform) or tiny; maybe later                            |
| iannuttall/claude-agents, steipete/agent-rules                      | archived                                                                 |
| 0xfurai/…, lst97/…, vijaythecoder/…, blefnk/awesome-cursor-rules    | inactive for over six months                                             |
| agentsmd/agents.md                                                  | the AGENTS.md specification, not a collection                            |

No licensed, active collection of `AGENTS.md`-style prompts or Gemini subagents was found;
the importer still reads both formats from custom sources.

## Formats

| Format           | Files                       | Becomes                                                        |
| ---------------- | --------------------------- | -------------------------------------------------------------- |
| `claudeSubagent` | `.md` with YAML frontmatter | an agent (frontmatter `tools`, `model` -> Claude Code options) |
| `claudeSkill`    | a folder with `SKILL.md`    | a skill (all files of the folder)                              |
| `cursorRule`     | `.mdc` / `.md` rules        | a rule (`globs`, `alwaysApply` -> Cursor options)              |
| `codexAgent`     | `.toml` Codex agents        | an agent (`model`, reasoning effort, sandbox -> Codex options) |
| `geminiCommand`  | `.toml` custom commands     | a Gemini command                                               |
| `geminiAgent`    | `.md` Gemini subagents      | an agent                                                       |
| `agentsMd`       | `AGENTS.md`-style prompts   | an agent                                                       |

Custom sources (any public GitHub repository, optionally a folder in it) are labeled
"unverified"; the format of each file is guessed from its name and frontmatter.

## Import results

First full refresh on 2026-10-18 (27 GitHub API calls of the unauthenticated 60/hour):

| Source              | Imported | Unsupported                                      |
| ------------------- | -------- | ------------------------------------------------ |
| anthropic-skills    | 13       | 1 (`claude-api`: over 100 000 characters)        |
| openai-skills       | 26       | 1 (`cloudflare-deploy`: 311 files, over the cap) |
| superpowers         | 15       | 0                                                |
| wshobson-agents     | 387      | 0                                                |
| voltagent-claude    | 163      | 0                                                |
| voltagent-codex     | 175      | 0                                                |
| awesome-cursorrules | 256      | 1 (an empty rule)                                |
| cursor-rules-mdc    | 241      | 0                                                |
| gemini-cli          | 24       | 0                                                |

Identical files that a source keeps in several places (wshobson's plugins share agents)
appear once.

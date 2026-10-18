# Changelog

All notable changes to AgentFlow are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-10-18

The first public release.

### Added

- **Multi-agent workspace:** run Claude Code, Codex, Gemini CLI, your shell or any command in
  terminal panels, side by side in a VS Code-style dockable layout that is remembered between
  restarts. Agent CLIs restored from the last session wait for you instead of starting on
  their own.
- **Pixel characters:** 15 hand-made 16x16 characters with color variants, one per agent. They
  think, work, celebrate and sleep with your agents, and a prompt bar lets you type to any
  agent and attach workspace files.
- **Files and editor:** a fast file tree with icons that follows changes on disk, quick open
  (Ctrl/Cmd+P), and an editor with syntax highlighting. Deleted files go to the system trash.
- **API chats and agents:** chat with Anthropic or any OpenAI-compatible provider (OpenAI,
  DeepSeek, Gemini, OpenRouter, Groq, Ollama or your own endpoint). API agents can read,
  edit and run commands in your workspace with clear tool cards, diffs, per-conversation
  permissions and undo. Dangerous commands always ask first.
- **Keys stay safe:** API keys are stored only in your system keychain, never in files.
- **Usage and costs:** token counts and costs for API chats and the Claude Code, Codex and
  Gemini CLIs, with daily and monthly budgets, a dashboard and context compaction.
- **Agent installer:** create an agent once and install it for Claude Code, Codex, Gemini CLI,
  Cursor and AgentFlow, with a preview of every file before anything is written.
- **Community library:** browse about 1,300 agents, skills, rules and commands from curated
  open-source collections, with licenses, a safety scan and one-click install.
- **Workstreams and the Agent Stage:** a manager agent hands tasks to worker agents and reads
  their reports, manually, step by step or automatically, with a stage where you watch the
  handoffs happen.
- **Look and feel:** VS Code theme import, three built-in themes, a polished design with
  smooth (reduced-motion aware) animations, and five languages: English, Türkçe, Deutsch,
  中文 and Русский.
- **First-run guide:** pick your language, theme and character, connect an AI and open a
  folder in a minute; it can be opened again from Settings.
- **Updates:** AgentFlow checks for new versions on startup (can be turned off) and installs
  them only when you click "Install and restart". Every update is signature-checked.

[Unreleased]: https://github.com/realrevi/agentflow/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/realrevi/agentflow/releases/tag/v0.1.0

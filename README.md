# AgentFlow

**A multi-agent terminal and AI manager for vibe coders.** Run several AI coding agents side by
side in a VS Code-style dockable layout, watched over by cute pixel-art agent characters.

AgentFlow is a lightweight desktop app (Tauri 2 + Rust + SolidJS) for Windows, macOS and Linux.
The goals are low RAM, a fast UI, a clean codebase, and a fun look.

> **Status:** early development. Phases 1–8 of 12 are done: app shell, themes, languages,
> dockable layout, real terminal panels, the workspace file tree, a code editor, pixel
> characters with a prompt bar, API agents with tools, token and cost tracking, and an agent
> installer. See the [roadmap](#roadmap).

![AgentFlow screenshot](docs/screenshot.png)
<!-- Phase 5: an API chat panel with its character and prompt bar; open panels and saved chats in the Agents view. -->

## Features

Available now (phases 1–6):

- VS Code-style shell: activity bar, resizable and collapsible sidebar, dock area, status bar
- Dockable panels (split, tab, drag, close, re-open from **+**), and the layout persists across restarts
- Theme engine that reads **VS Code color themes** (`.json`, comments allowed); import your favourite
- Built-in themes: **AgentFlow Dark** (default), **AgentFlow Light**, **Retro CRT** (amber)
- User overrides on top of any theme: text color, accent color, and the glowing "agent bold" style
- Five languages, switchable live: English, Türkçe, Deutsch, 中文, Русский
- Offline-first: fonts are bundled (JetBrains Mono, Pixelify Sans), no network needed
- Terminal panels (xterm.js, WebGL when visible): your shell, Claude Code, Codex, Gemini CLI
  or any custom command, each in its own dockable panel; agent CLIs missing from PATH are
  shown as unavailable
- Workspace folder: new terminals start there; recent folders in the status bar
- Restored agent panels wait for you to press Start, so they never spend tokens on launch
- Default layout: agents and editors in the center, the file tree on the right, shells at the
  bottom. Each area remembers its role, so new panels land where you expect
- File tree that respects `.gitignore`, updates live while agents change files, and stays
  smooth with 10k+ entries; create, rename, delete (to the OS trash) and copy paths
- Drag a file from the tree onto a terminal to type its path for your agent
- CodeMirror 6 editor (loaded on demand) with syntax highlighting from your theme, safe saves
  that never overwrite changes made on disk, and a quick open palette (Ctrl/Cmd+P)
- 15 pixel characters: every agent panel gets one (in its tab, its corner and the Agents list);
  it thinks, works, finishes and sleeps along with the agent, and rides above the terminal
  cursor while output streams. Pick another one from the tab or the Characters view
- A prompt bar under agent panels: Enter sends straight to the agent, Shift+Enter adds a line,
  Up recalls; your character walks along as you type and catches the prompt as a pixel packet.
  Prompt history per panel and workspace (click the character)
- Animations cost nothing while agents rest, stop when hidden and follow reduced motion
- API agents: Anthropic, OpenAI, DeepSeek, Gemini, OpenRouter, Groq, a local
  Ollama or any OpenAI-compatible server, each chat in its own panel with a character. Models
  come from the provider (or type one in); add providers in Settings › Providers and test them
- Replies stream in smoothly as Markdown, with copyable code blocks; Stop at any time, Retry
  after an error. Drag files from the tree onto the prompt bar to send them as context
- Chats are saved per workspace and listed in the Agents view: reopen, rename or delete them.
  Token usage is recorded with every reply
- API keys live only in your OS keychain, never in settings files or logs; the app only ever
  shows their last 4 characters
- Token and cost tracking for API chats **and** your agent CLIs: Claude Code, Codex and Gemini
  CLI usage is read from their own session logs (read-only), so it counts even outside
  AgentFlow. A Usage view and dashboard panel show today / 7 / 30 days, a daily chart by
  source, breakdowns by model, workspace and agent panel, and what prompt caching saved
- Every agent panel shows its session tokens and cost next to its character; today's total is
  in the status bar. Prices ship with their source and date; edit them, add custom models,
  mark a provider free, or label CLI costs "API-equivalent" if a subscription pays for them
- Daily and monthly budgets (overall and per provider) warn at 80% and 100%; an optional hard
  stop pauses API chats until you allow a send once (CLI agents can only be warned)
- Chats show a live pre-send estimate, cache hits, and a context-limit warning with
  **Compact**: older turns become a summary written by a model you choose (or automatically)

- API agents work in your workspace with seven tools: list, read, find and search files,
  write and edit files, and run commands (no network tools). Each call is a compact card with
  its result; reads group into one "Explored" row, and the model's reasoning streams into a
  collapsible "Thinking" section
- You stay in control: **Ask** (reads run, edits and commands ask), **Auto-edit** or
  **Read-only** per chat. Edits ask with a real diff; dangerous commands (recursive deletes,
  force pushes, piping downloads into a shell, ...) always ask, with a red badge. Deny with a
  reason and the model gets it. Every turn that changed files can be undone in one click
- A default system prompt tells the model about your OS, shell, workspace and its
  `AGENTS.md` / `CLAUDE.md` (can be turned off); files an agent changed are marked in the tree

![An API agent asking to edit a file](docs/agent.png)

![The usage dashboard](docs/usage.png)

### Agent installer

Set up the same agent in every tool you use, from one template. Open the **Agents library**
(activity bar, **+**, or `>` in Ctrl/Cmd+P):

- Start from 8 starters (Code Reviewer, UI/Visual Tester, Test Writer, Bug Hunter, Refactorer,
  Docs Writer, Security Reviewer, Commit & PR Writer), import a template file, or describe
  the agent and let one of your models draft it ("Create with AI")
- Install it for **Claude Code** (subagent), **Codex CLI** (subagent), **Gemini CLI**
  (subagent), **Cursor** (project rule) and **AgentFlow** (an API agent with its own role,
  tools and character), in the project or, where a tool documents it, your home folder
- Answers are prefilled from your workspace (languages, package manager, tests, UI framework,
  brand colors, fonts, spacing, breakpoints), read locally and marked "detected"
- Every file is previewed with its path and a diff before anything is written. `CLAUDE.md`,
  `AGENTS.md` and `GEMINI.md` only get a marked section; your own text stays as it is
- Update, change answers or uninstall later; files you edited are never overwritten or deleted
  without asking. Templates are plain JSON ([format](docs/agent-template.md), [where each tool
  looks](docs/agent-targets.md)) and can be exported and shared

![Installing an agent: the preview with a diff](docs/installer.png)

![The 15 characters](docs/characters.png)

Planned:

- A community agent library

## Roadmap

| Phase | Scope                                                                  | Status |
| ----- | ---------------------------------------------------------------------- | ------ |
| 1     | Skeleton, theme engine, i18n, dockable layout                          | Done   |
| 2     | PTY + multiple terminal panels (xterm.js WebGL, portable-pty)          | Done   |
| 3     | Workspace file tree (notify + ignore) + CodeMirror 6 viewer/editor     | Done   |
| 4     | Pixel character system (15 characters, 16x16 JSON sprites, speech bar) | Done   |
| 5     | AI provider APIs + API chat panels (keys in the OS keychain)           | Done   |
| 6     | Token and cost tracking (API chats + agent CLI logs, budgets)          | Done   |
| 7     | API agent tools + default system prompt                                | Done   |
| 8     | Agent installer wizard (per-AI formats and paths)                      | Done   |
| 9     | Manager–worker workstreams + the Agent Stage                           |        |
| 10    | Community agent library from GitHub                                    |        |
| 11    | UI polish                                                              |        |
| 12    | Cross-platform builds + CI                                             |        |

## Keyboard shortcuts

| Shortcut (Cmd on macOS)     | Action                       |
| --------------------------- | ---------------------------- |
| Ctrl+P                      | Quick open a workspace file  |
| Ctrl+P, then `>`            | Commands (Agents library...) |
| Ctrl+S                      | Save the current editor      |
| Ctrl+Shift+`                | New shell                    |
| Ctrl+Shift+C / Ctrl+Shift+V | Copy / paste in a terminal   |
| Enter / Shift+Enter         | Send / new line (prompt bar) |
| Up / Down (prompt bar)      | Recall earlier prompts       |
| F2 / Delete (file tree)     | Rename / move to the trash   |

## Development setup

You need **Rust (stable)**, **Node.js 20+** and **pnpm 10+**, plus the Tauri system dependencies
for your OS ([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)).

### Windows

1. Install [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
   with the "Desktop development with C++" workload.
2. WebView2 ships with Windows 10 (1803+) and 11. Otherwise install the
   [Evergreen runtime](https://developer.microsoft.com/microsoft-edge/webview2/).
3. Install Rust with [rustup](https://rustup.rs) (MSVC toolchain), Node.js, then `npm i -g pnpm`.

### macOS

```sh
xcode-select --install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
brew install node pnpm
```

### Linux

Debian / Ubuntu:

```sh
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev \
  build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

Fedora:

```sh
sudo dnf install webkit2gtk4.1-devel libsoup3-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel
sudo dnf group install "c-development"
```

Arch:

```sh
sudo pacman -S --needed webkit2gtk-4.1 libsoup3 base-devel curl wget file openssl \
  appmenu-gtk-module libappindicator-gtk3 librsvg xdotool
```

Then install Rust with [rustup](https://rustup.rs), Node.js and pnpm. API keys are stored through
the Secret Service, so a keyring daemon (GNOME Keyring, KWallet...) must be running.

### Run

```sh
pnpm install
pnpm tauri dev          # desktop app with hot reload
```

### Checks

```sh
pnpm typecheck          # TypeScript (strict)
pnpm test               # frontend unit tests (vitest)
pnpm build              # production frontend bundle
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test              # backend unit tests
cargo build             # compile check without bundling
cd ..
pnpm tauri build --debug  # full desktop build + installers
```

## Project layout

See [CLAUDE.md](CLAUDE.md) for the folder map, code rules and conventions.

## License

[MIT](LICENSE) © AgentFlow contributors. Bundled fonts are under the SIL Open Font License 1.1
(see `src/assets/fonts/*-OFL.txt`).

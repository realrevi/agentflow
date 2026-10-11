# AgentFlow

**A multi-agent terminal and AI manager for vibe coders.** Run several AI coding agents side by
side in a VS Code-style dockable layout, watched over by cute pixel-art agent characters.

AgentFlow is a lightweight desktop app (Tauri 2 + Rust + SolidJS) for Windows, macOS and Linux.
The goals are low RAM, a fast UI, a clean codebase, and a fun look.

> **Status:** early development. Phases 1–3 of 9 are done: app shell, themes, languages,
> dockable layout, real terminal panels, the workspace file tree and a code editor. See the
> [roadmap](#roadmap).

![AgentFlow screenshot](docs/screenshot.png)
<!-- Phase 3: editor in the center, file tree on the right, a real shell below (Linux, WebKitGTK). -->

## Features

Available now (phases 1–3):

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

Planned:

- 15 pixel characters with a typewriter speech bar
- AI providers: Claude Code, Codex and Gemini CLIs, plus Anthropic, OpenAI, Gemini, DeepSeek and
  OpenAI-compatible APIs (keys stored in the OS keyring)
- Token and cost tracking
- Agent installer wizard and a community agent library

## Roadmap

| Phase | Scope                                                                  | Status |
| ----- | ---------------------------------------------------------------------- | ------ |
| 1     | Skeleton, theme engine, i18n, dockable layout                          | Done   |
| 2     | PTY + multiple terminal panels (xterm.js WebGL, portable-pty)          | Done   |
| 3     | Workspace file tree (notify + ignore) + CodeMirror 6 viewer/editor     | Done   |
| 4     | Pixel character system (15 characters, 16x16 JSON sprites, speech bar) |        |
| 5     | AI providers (CLI agents + HTTP APIs, keys in the OS keyring)          |        |
| 6     | Token and cost tracking                                                |        |
| 7     | Agent installer wizard (per-AI formats and paths)                      |        |
| 8     | Community agent library from GitHub                                    |        |
| 9     | Cross-platform builds + CI                                             |        |

## Keyboard shortcuts

| Shortcut (Cmd on macOS)     | Action                      |
| --------------------------- | --------------------------- |
| Ctrl+P                      | Quick open a workspace file |
| Ctrl+S                      | Save the current editor     |
| Ctrl+Shift+`                | New shell                   |
| Ctrl+Shift+C / Ctrl+Shift+V | Copy / paste in a terminal  |
| F2 / Delete (file tree)     | Rename / move to the trash  |

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

Then install Rust with [rustup](https://rustup.rs), Node.js and pnpm.

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

# AgentFlow

Open-source (MIT) desktop app for vibe coders: a multi-agent terminal and AI manager with a
VS Code-style dockable layout and cute pixel-art agent characters. Targets Windows, macOS and
Linux. Priorities: **low RAM, fast, clean code organization, fun look.**

## Stack (fixed)

- **Backend:** Tauri 2, Rust stable, tokio, serde/serde_json, thiserror, tracing, portable-pty,
  which, ignore + notify, trash, reqwest (rustls + ring, no default features), keyring (OS
  keychain); tauri plugins: dialog, clipboard-manager
- **Frontend:** SolidJS + TypeScript (strict) + Vite, pnpm, xterm.js (+ fit, webgl, unicode11),
  CodeMirror 6 (languages per file type), marked + DOMPurify (chat only); all three lazy
- **Layout/styling:** `dockview-core` behind a thin Solid wrapper (stylesheet from `dockview`,
  both pinned to the same version); plain CSS with `--af-*` variables, no React, Electron,
  Tailwind or UI kits; bundled fonts JetBrains Mono (code), Pixelify Sans (small accents only).

## Folder map

```
src-tauri/src/
  main.rs, lib.rs     app setup, managed state, command registration; error.rs: AppError
  settings/           versioned settings JSON in the OS app config dir (atomic write, migrations)
  themes/             built-in + imported VS Code themes: JSONC, validation, syntax colors, storage
  commands/           thin #[tauri::command] fns that delegate to modules
  pty/                PtyManager, sessions (reader/waiter/pump threads), shells, CLI detection
  fs/                 workspace: path checks, list/read/write/trash, quick open index, watcher
  prompts/            prompt history per workspace (app data dir)
  providers/          Provider trait, Anthropic + OpenAI-compatible adapters, SSE, keys, errors
  conversations/      API chats per workspace (app data dir, one JSON file per conversation)
  chat/               ChatManager (one stream per chat, cancel), request building (blocks, cache
                      breakpoints, attachment dedup), Compact, pre-send estimate, Channel events
  agent/              API agent loop, tools/, permissions, dangerous commands, checkpoints, prompt
  tokens/             UsageTracker: JSONL ledger, buckets, prices, budgets, CLI log readers
  util/               atomic_write, run_blocking, lock/Signal, workspace_key, hex color, test_dir
src/
  main.tsx, App.tsx   entry: bootstrap (settings -> locale -> theme -> workspace fs), then render
  app/                shell: ActivityBar, Sidebar (+ views), StatusBar, Toasts, Dialogs, styles/
  features/layout/    dockview host, panels, roles, placement, saved layout, tabs, characters
  features/terminal/  TerminalController, lazy TerminalPanel, agent activity
  features/chat/      lazy ChatPanel: session, agent turn, smooth reveal, markdown, attachments
  features/agentui/   corner character overlay + prompt bar, shared by terminal and chat panels
  features/providers/ presets (base URLs), model list and key status stores
  features/characters/  sprites/*.json, renderer, shared rAF animator, Avatar, state heuristic
  features/workspace/ workspace folder, fs session (backend open + watcher events), path helpers
  features/files/     Files panel: tree model/store, virtual TreeView, icons, quick open
  features/editor/    lazy EditorPanel + EditorController (CodeMirror), languages, dirty store
  features/theme/     VS Code -> --af-* mapping (+ syntax vars), user overrides, engine, store
  features/i18n/      typed t(), lazy locale loading, reportError()
  features/settings/  settings store (debounced save) + lazy SettingsPanel and sections
  features/tokens/    usage store (one Channel), formats, badge, chart, Usage view + lazy panel
  features/agents/, src-tauri/src/agents/: empty stubs (installer, library)
  shared/             ipc (+ ipcCommands), types (+ chat/usage/agentTypes), errors, toast, dialog, ui/
  locales/            en.json (source of truth), tr, de, zh, ru
themes/               built-in theme JSON (embedded into the Rust binary via include_str!)
```

Layering: `shared/` imports nothing from `features/` or `app/`. Core features (`i18n`,
`settings`, `theme`, `characters`, `agentui`, `providers`, `tokens`) never import panel features. Panel
features (`terminal`, `files`, `editor`, `chat`) import core features and `workspace`, never each
other. `layout` and the settings panel compose panel features; `app/` composes all.

## Code rules

- Small files, one responsibility each; nothing over ~300 lines. No `any` in TS, no
  `unwrap()`/`expect()` in non-test Rust. Errors flow as `AppError` -> `reportError()` toast.
- All IPC goes through `src/shared/ipc.ts`; components never call `invoke` directly.
- Minimal dependencies; justify every new crate/package in the commit message.
- Performance: no polling (events, ResizeObserver, debounced saves), fine-grained signals (big
  blobs like the layout or tree listings stay out of reactive stores), lazy-load heavy panels.
- Rust file I/O and keychain calls in commands run through `util::run_blocking`.

## IPC, PTY and FS patterns

- Rust: a fn in the owning module, a thin `#[tauri::command] async fn` in `commands/<area>.rs`
  returning `AppResult<T>`, registered in `lib.rs`. TS: an entry in `shared/ipcCommands.ts`, a
  wrapper on `ipc`, structs mirrored in `shared/types.ts` (serde camelCase). Errors arrive as
  `AppError { kind, message }` (`tooLarge` also has `size`); `toAppError()` normalizes them.
- Streams use a `Channel` created _before_ the invoke, never global events. PTY: raw bytes
  (coalesced ~10 ms / 64 KB, `OutputDecoder`) then `{ type: 'exit', code, signal }`; flow control
  via `pty_ack` (reader pauses at 512 KB). Kill = SIGHUP / TerminateProcess, SIGKILL after 2 s.
- Panels persist _config_ only (incl. `character`, `historyKey`). Restored agent CLIs never
  auto-start; shells follow `terminal.autoStartShells`.
- Frontend paths are workspace-relative with `/`; `fs/paths.rs` rejects `..`, absolute paths and
  escaping symlinks. One `ignore` config (`fs/walk.rs`: .gitignore even outside git, `.git`
  hidden, dotfiles per `files.showHidden`) for list, quick open and the watcher (notify, 200 ms
  debounce, `{ paths, structural, rescan }` batches fanned out by `workspace/fsSession.ts`).
- Saves are atomic with the mtime the file was read at (mismatch = `conflict`); deletes go to the
  OS trash; reads stop at 5 MB.

## Providers & API chat

- **Adapters:** `trait Provider { stream_chat, list_models }`; `anthropic.rs` (Messages) or
  `openai.rs` (Chat Completions; a `Preset` holds vendor quirks, e.g. the reasoning echo). A
  `Decoder` turns SSE into deltas, reasoning, tool calls, usage and a stop reason; `http.rs` maps
  errors to `ProviderError` kinds. A new OpenAI-compatible vendor is a preset. No hardcoded models.
- **Keys live ONLY in the OS keychain** (`keys.rs`, service `dev.agentflow.desktop`, account
  `provider:<id>`; tests use `MemoryStore`), never in settings, logs, error text or IPC (the
  frontend gets `KeyStatus { hasKey, last4 }`). Errors are `redacted()`, redirects not followed,
  base URLs HTTPS (plain HTTP only for loopback, `url.rs`).
- `chat_send(…, Channel)`: one turn per chat, `chat_cancel`, all cancelled on reload/exit; every
  model call is saved with its usage. Rust tests use `providers/tests/mock.rs` (never real APIs
  or keys; `refusing_port()` for dead servers). Reveal: `chat/reveal.ts`; Markdown: DOMPurify.

## API agents (phase 7)

- Loop (`agent/run.rs`), up to `agent.maxIterations` calls per prompt: model call (`step.rs`)
  stored as an assistant message with `parts` (reasoning, text, toolCall) -> per call: permission
  and run (`execute.rs`) -> ONE `tool` message with the results -> next call. Cancel ends the
  stream, a permission wait and a running command (process tree killed); calls that never ran
  get "not run" results. Tools rejected by the provider: model remembered (`model-tools.json`,
  settings override wins), the chat goes on without tools + `notice`.
- Conversations v2: `parts`, `prefix` (digest of system + tools; Anthropic thinking is replayed
  only under the same prefix and never before a compaction), `agent` (mode, allowed tools, tools
  off, the environment text, built once so the cached prefix stays stable).
- Tools (`agent/tools/`): an entry in `TOOLS` (name, read|write|command, JSON schema) + a fn.
  Paths only via `WorkspaceRoot`; output capped (`agent.toolResultCap`); writes go through
  `save()`, which checkpoints the original once per turn (`checkpoint.rs`). No network tools.
- Permissions (`permissions.rs`): Ask (reads run), Auto-edit (writes run), Read-only (write and
  command tools not offered); "allow for this conversation" is per tool. **Dangerous commands
  ALWAYS ask**: patterns live only in `agent/dangerous.rs`, each with Unix/Windows tests.
- Events: `reasoning`, `status { phase }`, `toolStart`, `toolEnd { callId, ok, summary }`,
  `permission { request }`, `notice`, `message` (a stored step); answer with
  `agent_permission_answer`. Frontend: `chat/agentTurn.ts`, `PermissionRow`, `AgentControls`.

## Usage tracking (phase 6)

- `tokens/tracker.rs`: monthly JSONL ledger in `<app data>/usage` (append-only, dedup by `id`),
  15-minute buckets for 62 days, cost at query time (`pricing.json`, OpenRouter cache,
  overrides; no price = "—"). Lock order: `cli` before `state`. CLI logs are tailed from saved
  byte offsets and watched with notify; `pty_spawn(panel)` matches CLI usage to its panel.
- ONE `usage_subscribe` Channel (`tokens/usageStore.ts`): `changed { panels }`, `repriced`,
  `budget { status }`. Hard stop = API chats only (kind `budget`, "Allow once" token -> resend).
  Chat: debounced `usage_estimate`, Compact at the threshold; Anthropic cache: max 4 breakpoints.

## Layout roles

- Groups have a role (`layout/roles.ts`): `agents` (center), `files` (right, ~22%), `terminal`
  (~30% under the center). New panels open in the last focused group of their role
  (`placement.ts`). Saved layout `{ version: 2, dock, roles }`; bump `LAYOUT_VERSION` on changes.

## Characters & prompt bar

- Sprites: `characters/sprites/*.json` = `{ id, name, size, palettes, frameMs, states }`, 8 states
  (idle walk think work receive done error sleep) of 2-4 frames, `.` = transparent, else a
  base-36 palette index (`validate.ts`). Names are never translated.
- `renderer.ts` caches ImageBitmap strips; all motion uses the ONE loop in `animator.ts` (runs
  only while something animates on screen). Respect `reducedMotion()`.
- Terminal (`stateMachine.ts`): prompt -> think, output -> work, 2 s quiet -> done, 5 s -> idle,
  exit 0 -> sleep else error. Chat: send -> think, delta/tool -> work (bubble = tool status).

## Themes, i18n, settings

- VS Code `colors` -> `--af-*` (`theme/mapping.ts`), `tokenColors` -> `--af-syntax-*`; never
  hard-code colors. Overrides (text, accent, agent bold `#F5C542`, `--af-glow`): `overrides.ts`.
- Flat dotted keys in `src/locales/en.json`; `I18nKey = keyof typeof en`; every visible string
  uses `t('key', { param })`. New keys go into **all five** locales (typecheck + tests check).
- Rust owns `settings.json` (v7: `agent`): bump `CURRENT_VERSION` + a `migrate.rs` step on shape
  changes; `normalized()` repairs values; `defaults.test.ts` checks TS defaults against Rust.

## Commands

```sh
pnpm install && pnpm tauri dev
pnpm typecheck && pnpm test && pnpm build
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cargo build    # in src-tauri: compile check, no bundling
```

## Roadmap (11 phases)

1. [x] Skeleton, theme engine, i18n, dockable layout
2. [x] PTY + multi terminal panels (xterm.js WebGL, portable-pty)
3. [x] Workspace file tree (notify + ignore) + CodeMirror 6 viewer/editor
4. [x] Pixel character system (15 characters, 16x16 JSON sprites scaled, typewriter speech bar)
5. [x] AI provider APIs (Anthropic + OpenAI-compatible, keychain keys) + API chat panels
6. [x] Token & cost tracking (ledger, CLI log readers, prices, budgets, dashboard, Compact)
7. [ ] API agent tools + default system prompt (7A: loop, tools, permissions, minimal UI done)
8. [ ] Agent installer wizard (per-AI formats/paths)
9. [ ] Community agent library from GitHub
10. [ ] UI polish
11. [ ] Cross-platform builds + CI

Only build what the current phase lists; leave future modules as empty stubs.

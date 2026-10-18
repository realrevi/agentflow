# AgentFlow

Open-source (MIT) desktop app for vibe coders: a multi-agent terminal and AI manager with a
VS Code-style dockable layout and cute pixel-art agent characters. Targets Windows, macOS and
Linux. Priorities: **low RAM, fast, clean code organization, fun look.**

## Stack (fixed)

- **Backend:** Tauri 2, Rust stable, tokio, serde/serde_json, thiserror, tracing, portable-pty,
  which, ignore + notify, trash, reqwest (rustls + ring, no default features), keyring (OS
  keychain); tauri plugins: dialog, clipboard-manager, opener, notification
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
  chat/               ChatManager (one stream per chat), request building, Compact, estimate
  agent/              API agent loop, tools/, permissions, dangerous commands, checkpoints, prompt
  agents/             installer: templates, per-tool renderers, sections, plan/apply, registry
  tokens/             UsageTracker: JSONL ledger, buckets, prices, budgets, CLI log readers
  turns/, workstreams/  CLI turn ends + activity from session logs; handoff log (JSONL)
  util/               atomic_write, run_blocking, lock/Signal, workspace_key, hex color, test_dir
src/
  main.tsx, App.tsx   entry: bootstrap (settings -> locale -> theme -> workspace fs), then render
  app/                shell: ActivityBar, Sidebar (+ views), StatusBar, Toasts, Dialogs, styles/
  features/layout/    dockview host, panels, roles, placement, saved layout, tabs, characters
  features/terminal/  TerminalController, lazy TerminalPanel, agent activity
  features/chat/      lazy ChatPanel: session, agent/ (timeline, cards, undo), reveal, markdown
  features/agentui/   corner overlay + prompt bar; activity, turn ends, attention, ports, input
  features/providers/ presets (base URLs), model list and key status stores
  features/characters/  sprites/*.json, renderer, shared rAF animator, Avatar, state heuristic
  features/workspace/ workspace folder, fs session (backend open + watcher events), path helpers
  features/files/     Files panel: tree model/store, virtual TreeView, icons, quick open
  features/editor/    lazy EditorPanel + EditorController (CodeMirror), languages, dirty store
  features/theme/     VS Code -> --af-* mapping (+ syntax vars), user overrides, engine, store
  features/i18n/      typed t(), lazy locale loading, reportError()
  features/settings/  settings store (debounced save) + lazy SettingsPanel and sections
  features/tokens/    usage store (one Channel), formats, badge, chart, Usage view + lazy panel
  features/agents/    lazy Agents library panel (wizard/, installed list), profiles store
  features/workstreams/ protocol, engine, store, runtime, cards, sidebar section, lazy Timeline
  features/stage/     lazy Agent Stage: layout, positions, scene, motion, canvas renderer
  shared/             ipc (+ ipcCommands), types (+ chat/usage/agentTypes), errors, toast, dialog, ui/
  locales/            en.json (source of truth), tr, de, zh, ru
themes/               built-in theme JSON (embedded into the Rust binary via include_str!)
```

Layering: `shared/` imports nothing from `features/` or `app/`. Core features (`i18n`,
`settings`, `theme`, `characters`, `agentui`, `providers`, `tokens`) never import panel features. Panel
features (`terminal`, `files`, `editor`, `chat`) import core features and `workspace`, never each
other. `workstreams` and `stage` import core features only (the Stage gets `StageHooks`);
`layout` and the settings panel compose panel features; `app/` composes all.

## Code rules

- Small files, one responsibility each; nothing over ~300 lines. No `any` in TS, no
  `unwrap()`/`expect()` in non-test Rust. Errors flow as `AppError` -> `reportError()` toast.
- All IPC goes through `src/shared/ipc.ts` (never `invoke` in components). Minimal
  dependencies; justify every new crate/package in the commit message.
- Performance: no polling (events, ResizeObserver, debounced saves), fine-grained signals (big
  blobs like the layout or tree listings stay out of reactive stores), lazy-load heavy panels.
  Rust file I/O and keychain calls in commands run through `util::run_blocking`.

## IPC, PTY and FS patterns

- Rust: a fn in the owning module, a thin `#[tauri::command] async fn` in `commands/<area>.rs`
  returning `AppResult<T>`, registered in `lib.rs`. TS: an entry in `shared/ipcCommands.ts`, a
  wrapper on `ipc`, structs mirrored in `shared/types.ts` (serde camelCase). Errors arrive as
  `AppError { kind, message }` (`tooLarge` also has `size`); `toAppError()` normalizes them.
- Streams use a `Channel` created _before_ the invoke, never global events. PTY: raw bytes
  (coalesced ~10 ms / 64 KB, `OutputDecoder`) then `{ type: 'exit', code, signal }`; flow control
  via `pty_ack` (reader pauses at 512 KB). Kill = SIGHUP / TerminateProcess, SIGKILL after 2 s.
  Panels persist _config_ only; restored agent CLIs never auto-start.
- Frontend paths are workspace-relative with `/` (`fs/paths.rs` rejects `..`, absolute paths,
  escaping symlinks). One `ignore` config (`fs/walk.rs`) for list, quick open and the watcher
  (200 ms batches via `workspace/fsSession.ts`). Saves are atomic with the read mtime
  (`conflict`), deletes go to the OS trash, reads stop at 5 MB.

## Providers & API chat

- **Adapters:** `trait Provider { stream_chat, list_models }`; `anthropic.rs` (Messages) or
  `openai.rs` (Chat Completions; a `Preset` holds vendor quirks, e.g. the reasoning echo). A
  `Decoder` turns SSE into deltas, reasoning, tool calls, usage and a stop reason; `http.rs` maps
  errors to `ProviderError` kinds. A new OpenAI-compatible vendor is a preset. No hardcoded models.
- **Keys live ONLY in the OS keychain** (`keys.rs`, service `dev.agentflow.desktop`, account
  `provider:<id>`; tests use `MemoryStore`), never in settings, logs, error text or IPC (the
  frontend gets `KeyStatus { hasKey, last4 }`). Errors are `redacted()`, redirects not followed,
  base URLs HTTPS (plain HTTP only for loopback, `url.rs`).
- `chat_send(…, Channel)`: one turn per chat, cancelled on reload/exit; tests use `mock.rs`.

## API agents (phase 7)

- Loop (`agent/run.rs`, up to `agent.maxIterations` calls): model call (`step.rs`, stored with
  `parts`) -> per call permission + run (`execute.rs`) -> ONE `tool` message -> next call. Cancel
  ends stream, permission wait and command (process tree); unrun calls get "not run" results.
  Tools rejected by the provider: model remembered (`model-tools.json`), chat goes on without.
- Conversations v2: `parts`, `prefix` (system + tools digest), `agent` (mode, grants, profile).
- Tools (`agent/tools/`): an entry in `TOOLS` (name, read|write|command, JSON schema) + a fn.
  Paths only via `WorkspaceRoot`; output capped (`agent.toolResultCap`); writes go through
  `save()`, which checkpoints the original once per turn (`checkpoint.rs`). No network tools.
- Permissions (`permissions.rs`): Ask (reads run), Auto-edit (writes run), Read-only (write and
  command tools not offered); "allow for this conversation" is per tool. **Dangerous commands
  ALWAYS ask** (never a conversation grant): patterns + reason ids only in `agent/dangerous.rs`,
  each with tests and an `agent.danger.<id>` text.
- **Permission answers** come only from a trusted click/keypress on that request's card
  (`liveTurn.answer`: `isTrusted`, pending id) and carry the conversation id; the gate rejects
  unknown, answered or foreign ids. A turn that ends (cancel, panel close, reload) never allows.
- UI: `chat/agent/` (`timeline.ts` cards, `liveTurn.ts` in flight); the layout lends `ChatHooks`.

## Agent installer (phase 8)

- One template (`agents/template.rs`, schema 1; docs/agent-template.md + the JSON Schema in
  `schemas/`, a test keeps them in step) -> `targets.rs` renders exact files per tool
  (docs/agent-targets.md: Claude/Gemini subagent .md, Codex subagent .toml, Cursor .mdc,
  `.agentflow/agents/<id>.md`). Placeholders are plain substitution; unknown ones are errors.
- **Shared files** (CLAUDE.md, AGENTS.md, GEMINI.md) only get a section between
  `<!-- agentflow:begin <id> -->` / `end` markers (`section.rs`): nothing outside is touched,
  CRLF kept, idempotent. Every install is planned and previewed first (`plan.rs`); files we did
  not create or that changed since (hash in `.agentflow/installed.json`) need a confirmation;
  user scope only into documented home folders, after its own confirmation (`roots.rs`).
  Profiles narrow tools, never loosen the mode. New tool format = a target fn + snapshot test.

## Workstreams & Agent Stage (phase 9)

- **Turn ends** (docs/turn-signals.md): API on `stop`; Claude Code / Codex from session logs
  (`turns/`, `turns_subscribe`); Gemini / custom by the idle heuristic (approximate). PTYs drop
  a parent Claude session's markers (`pty/session_env.rs`); prompts: text, then Enter 80 ms later.
- **Protocol** (`protocol.ts`): one fenced `agentflow-send to="<name>"` / `-ask` / `-done` per
  manager reply (CommonMark fences, last wins); no/bad block or unknown worker = pause, never a
  guess. Reports capped at `reportCapKb`, whole in the log. API managers get `extraSystem`
  (never stored); CLI managers a setup message per CLI session.
- **Engine** (`engine.ts`, pure): manual / semi-auto / auto; auto stops on ask/done, any
  permission (never answered), errors, bad replies, max rounds, budgets, user typing, Stop.
  Deliveries only to members (`canRoute`) via `agentui` ports. Persisted in layout v3.
- **Glow** (`agentui/attention.ts`) clears on focus + interaction. **Stage**: one canvas on the
  shared animator, idle = no frames, paused hidden/unfocused; reduced motion = no travel.

## Usage tracking (phase 6)

- `tokens/tracker.rs`: monthly JSONL ledger (append-only, dedup by `id`), 15-minute buckets,
  cost at query time (no price = "—"). Lock order: `cli` before `state`. CLI logs tailed from
  saved offsets; `pty_spawn(panel)` matches CLI usage to its panel. ONE `usage_subscribe`
  Channel; hard stop = API chats only ("Allow once" token). Anthropic cache: max 4 breakpoints.

## Layout, characters & prompt bar

- Groups have a role (`layout/roles.ts`): `agents` (center), `files` (right, ~22%), `terminal`
  (~30% under the center). New panels open in the last focused group of their role
  (`placement.ts`); a Stage moved left gets a `stage` group. Saved layout v3 (`dock`, `roles`,
  `workstreams`, `stage`; v2 upgrades as is); bump `LAYOUT_VERSION` on changes.
- Sprites: `characters/sprites/*.json`, 8 states (idle walk think work receive done error sleep)
  of 2-4 frames, `.` = transparent, else a base-36 palette index (`validate.ts`); names are
  never translated. ONE animation loop (`animator.ts`), only while visible; `reducedMotion()`.
- Terminal (`stateMachine.ts`): prompt -> think, output -> work, 2 s quiet -> done, 5 s -> idle,
  exit 0 -> sleep else error. Chat: send -> think, delta/tool -> work (bubble = tool status).

## Themes, i18n, settings

- VS Code `colors` -> `--af-*` (`theme/mapping.ts`), `tokenColors` -> `--af-syntax-*`; never
  hard-code colors. Overrides (text, accent, agent bold `#F5C542`, `--af-glow`): `overrides.ts`.
- Flat dotted keys in `src/locales/en.json`; `I18nKey = keyof typeof en`; every visible string
  uses `t('key', { param })`. New keys go into **all five** locales (typecheck + tests check).
- Rust owns `settings.json` (v8): bump `CURRENT_VERSION` + a `migrate.rs` step on shape changes;
  `normalized()` repairs values; `defaults.test.ts` checks TS defaults against Rust.

## Commands

```sh
pnpm install && pnpm tauri dev
pnpm typecheck && pnpm test && pnpm build
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cargo build    # in src-tauri: compile check, no bundling
```

## Roadmap (12 phases)

1. [x] Skeleton, theme engine, i18n, dockable layout
2. [x] PTY + multi terminal panels (xterm.js WebGL, portable-pty)
3. [x] Workspace file tree (notify + ignore) + CodeMirror 6 viewer/editor
4. [x] Pixel character system (15 characters, 16x16 JSON sprites scaled, typewriter speech bar)
5. [x] AI provider APIs (Anthropic + OpenAI-compatible, keychain keys) + API chat panels
6. [x] Token & cost tracking (ledger, CLI log readers, prices, budgets, dashboard, Compact)
7. [x] API agent tools + default system prompt (tool cards, diffs, permissions, undo)
8. [x] Agent installer wizard (per-AI formats/paths, templates, managed sections, detection)
9. [x] Manager–worker workstreams + the Agent Stage (turn detection, handoffs, glow, Stage)
10. [ ] Community agent library from GitHub
11. [ ] UI polish
12. [ ] Cross-platform builds + CI

Only build what the current phase lists; leave future modules as empty stubs.

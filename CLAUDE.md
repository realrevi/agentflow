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
  tokens/             UsageTracker: JSONL ledger, buckets, prices, budgets, CLI log readers
  util/               atomic_write, run_blocking, lock/Signal, workspace_key, hex color, test_dir
src/
  main.tsx, App.tsx   entry: bootstrap (settings -> locale -> theme -> workspace fs), then render
  app/                shell: ActivityBar, Sidebar (+ views), StatusBar, Toasts, Dialogs, styles/
  features/layout/    dockview host, panels, roles, placement, saved layout, tabs, characters
  features/terminal/  TerminalController, lazy TerminalPanel, agent activity
  features/chat/      lazy ChatPanel: session, smooth reveal, markdown, attachments, chat list
  features/agentui/   corner character overlay + prompt bar, shared by terminal and chat panels
  features/providers/ presets (base URLs), model list and key status stores
  features/characters/  sprites/*.json, renderer, shared rAF animator, Avatar, state heuristic
  features/workspace/ workspace folder, fs session (backend open + watcher events), path helpers
  features/files/     Files panel: tree model/store, virtual TreeView, icons, quick open
  features/editor/    lazy EditorPanel + EditorController (CodeMirror), languages, dirty store
  features/theme/     VS Code -> --af-* mapping (+ syntax vars), user overrides, engine, store
  features/i18n/      typed t(), lazy locale loading, reportError()
  features/settings/  settings store (debounced save) + lazy SettingsPanel and sections
  features/{tokens,agents}/, src-tauri/src/agents/: empty stubs (tokens UI: phase 6B)
  shared/             ipc (+ ipcCommands), types (+ chat/usageTypes), errors, toast, dialog, dnd, ui/
  locales/            en.json (source of truth), tr, de, zh, ru
themes/               built-in theme JSON (embedded into the Rust binary via include_str!)
```

Layering: `shared/` imports nothing from `features/` or `app/`. Core features (`i18n`,
`settings`, `theme`, `characters`, `agentui`, `providers`) never import panel features. Panel
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

- **Adapter pattern:** `trait Provider { stream_chat, list_models }` (`providers/provider.rs`);
  `build_provider(entry, key)` returns `anthropic.rs` (Messages) or `openai.rs` (Chat Completions;
  a `Preset` holds vendor quirks: system role, max-tokens field, `stream_options`, headers, model
  prefix). An adapter builds the request; its `Decoder` turns SSE events (`sse.rs`) into text
  deltas, usage and a stop reason. `http.rs` sends, pumps with cancel and maps HTTP/in-band errors
  to `ProviderError` kinds (auth, rateLimit + retryAfter, invalidModel, network, timeout...). A
  new OpenAI-compatible vendor is a preset; formats come from its docs. No hardcoded models.
- **Keys live ONLY in the OS keychain** (`keys.rs`: `SecretStore`, service `dev.agentflow.desktop`,
  account `provider:<id>`; tests use `MemoryStore`), never in settings, logs, error text or IPC:
  the frontend gets `KeyStatus { hasKey, last4 }`, the key field is write-only, removing a provider
  deletes its key. Errors are `redacted()` before leaving the backend, auth headers are sensitive,
  redirects are not followed. Base URLs: HTTPS; plain HTTP only for `localhost`/loopback (`url.rs`).
- `chat_send(…, Channel)` streams `delta | usage | stop | error` (one per chat, `chat_cancel`, all
  cancelled on reload/exit); the backend saves each reply with a `UsageRecord`, partial text too.
  Rust tests use the mock server in `providers/tests/mock.rs`, never real APIs or keys.
- Chat panels (kind `chat`) persist only their config. Reveal: `chat/reveal.ts` on the shared
  animator, ≤ ~300 ms behind, instant when `chat.smoothReveal` is off. Markdown: `markdown.ts`
  (raw HTML escaped, DOMPurify allowlist).

## Usage tracking (phase 6)

- `tokens/tracker.rs` (+ `queries.rs`): one `UsageTracker` in managed state. Ledger =
  `<app data>/usage/YYYY-MM.jsonl`, append-only, malformed lines skipped; records dedup by
  `id` (`chat:<conversation>:<message>`, CLI message/response ids). Totals live in 15-minute
  buckets for 62 days; costs are computed at query time (`pricing.rs`: `pricing.json` with
  source + date, OpenRouter prices from its models endpoint, user overrides, free providers;
  no price = "—", never guessed). Lock order: `cli` before `state`.
- Start (background thread): load ledger, backfill once from conversations (`.backfill`
  marker), read CLI logs changed in the last 62 days, watch the roots (notify). CLI readers
  (`tokens/cli/`) are read-only, tail from stored byte offsets (`cli-offsets.json`, saved only
  after the records are in the ledger). `pty_spawn(panel)` registers agent CLIs so usage
  matches a panel by CLI + cwd + time window (`panels.rs`), else its workspace.
- Frontend gets live changes on the Channel of `usage_subscribe` (`changed { panels }`,
  `repriced`, `budget { status }`); the usage commands take the local UTC offset. The hard
  budget stop blocks API chat sends only (`AppError::Budget`, kind `budget`);
  `usage_allow_once` returns a single-use token for `chat_send`'s `allowOnce`.
- Anthropic prompt caching (`chat/cache.rs`): at most 4 breakpoints (system prompt, end of
  request, end of the history before the newest prompt, newest attachment); keep the 20-block
  lookback in mind when adding blocks.

## Layout roles

- Groups have a role (`layout/roles.ts`): `agents` (center: agent CLIs, API chats, editors,
  settings), `files` (right, ~22%), `terminal` (shells, ~30% under the center). New panels open
  in the last focused group of their role; `placement.ts` recreates a missing one. Saved layout =
  `{ version: 2, dock, roles }`; bump `LAYOUT_VERSION` (`savedLayout.ts`) on incompatible changes.

## Characters & prompt bar

- Sprites: `characters/sprites/*.json` = `{ id, name, size, palettes, frameMs, states }`, 8 states
  (idle walk think work receive done error sleep) of 2-4 frames of `size` strings, `.` =
  transparent, else a base-36 palette index (`validate.ts`). Names are never translated. Agent
  panels (shells only with `characters.onShells`) get an unused one (`layout/characters.ts`).
- `renderer.ts` caches one ImageBitmap strip per (character, palette, scale, state). All motion
  uses the ONE loop in `animator.ts`, which runs only while something animates on screen in a
  visible document; idle/sleep are still frames. Respect `reducedMotion()`.
- Terminal heuristic (`stateMachine.ts`, `terminal/agentActivity.ts`): prompt -> think; output ->
  work; 2 s quiet -> done; 5 s later -> idle; exit 0 -> sleep, else error. Output within 400 ms of
  a prompt or keys is echo. Chat: send -> think, first delta -> work, stop -> done, error -> error.
  The prompt bar sends first; animations are decoration only.

## Themes, i18n, settings

- VS Code `colors` -> `--af-*` (`theme/mapping.ts`), `tokenColors` -> `--af-syntax-*`; never
  hard-code colors. Overrides (text, accent, agent bold `#F5C542`, `--af-glow`): `overrides.ts`.
- Flat dotted keys in `src/locales/en.json`; `I18nKey = keyof typeof en`; every visible string
  uses `t('key', { param })`. New keys go into **all five** locales (typecheck + tests check).
- Rust owns `settings.json` (v6, adds `usage` and the token options in `chat`): bump `CURRENT_VERSION` + add a
  `migrate.rs` step when the shape changes; `normalized()` repairs values; `defaults.test.ts`
  checks the TS defaults against Rust. The frontend uses `updateSettings()` (300 ms debounce).

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
6. [ ] Token & cost tracking (6A backend + IPC done; 6B: usage dashboard, badges, budgets UI)
7. [ ] API agent tools + default system prompt
8. [ ] Agent installer wizard (per-AI formats/paths)
9. [ ] Community agent library from GitHub
10. [ ] UI polish
11. [ ] Cross-platform builds + CI

Only build what the current phase lists; leave future modules as empty stubs.

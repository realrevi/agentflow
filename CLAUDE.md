# AgentFlow

Open-source (MIT) desktop app for vibe coders: a multi-agent terminal and AI manager with a
VS Code-style dockable layout and cute pixel-art agent characters. Targets Windows, macOS and
Linux. Priorities: **low RAM, fast, clean code organization, fun look.**

## Stack (fixed)

- **Backend:** Tauri 2, Rust stable, tokio, serde/serde_json, thiserror, tracing, portable-pty,
  which, ignore + notify, trash, reqwest (rustls + ring, no default features), keyring (OS
  keychain); tauri plugins: dialog, clipboard-manager, opener, notification
- **Frontend:** SolidJS + TypeScript (strict) + Vite, pnpm, xterm.js (+ fit, webgl, unicode11),
  CodeMirror 6 (languages per file type), marked + DOMPurify (`shared/markdown`, chat + library); all lazy
- **Layout/styling:** `dockview-core` behind a thin Solid wrapper (stylesheet from `dockview`,
  both pinned to the same version); plain CSS with `--af-*` variables, no React, Electron,
  Tailwind or UI kits; bundled fonts Inter (UI), JetBrains Mono (code/terminal only), Pixelify
  Sans (small accents only).

## Folder map

```
src-tauri/src/
  lib.rs, error.rs    app setup, state, command registration (commands/: thin wrappers); AppError
  settings/, themes/  versioned settings JSON (atomic, migrations); built-in + VS Code themes
  pty/                PtyManager, sessions (reader/waiter/pump threads), shells, CLI detection
  fs/                 workspace: path checks, list/read/write/trash, quick open index, watcher
  providers/          Provider trait, Anthropic + OpenAI-compatible adapters, SSE, keys, errors
  prompts/, conversations/  prompt history; API chats (app data dir, one JSON per conversation)
  chat/, agent/       ChatManager, requests, Compact; agent loop, tools/, permissions, undo
  agents/             installer: templates, per-tool renderers, skills, sections, plan/apply, registry
  library/            community library: registry, GitHub client, cache, importers, safety scan
  tokens/             UsageTracker: JSONL ledger, buckets, prices, budgets, CLI log readers
  turns/, workstreams/  CLI turn ends + activity from session logs; handoff log (JSONL)
  updates.rs, crash.rs  updater check/install (signed latest.json); panic hook -> crash.log
  util/               atomic_write, run_blocking, lock/Signal, workspace_key, hex color, test_dir
src/
  main.tsx, App.tsx   entry: bootstrap (settings -> locale -> theme -> workspace fs), then render
  app/                shell: ActivityBar, Sidebar (+ views), StatusBar, Toasts, Dialogs, styles/,
                      panelToggles (Ctrl/Cmd+B/J/Shift+E), onboarding/ (first-run), TipsCard
  features/layout/    dockview host, panels, roles, placement, saved layout, tabs, characters
  features/terminal/  TerminalController, lazy TerminalPanel; chat/: lazy ChatPanel, agent/ cards
  features/agentui/   corner overlay + prompt bar; activity, turn ends, attention, ports, input
  features/providers/ presets, models, key status; characters/: sprites, animator, Avatar
  features/workspace/ workspace folder, fs session (backend open + watcher events), path helpers
  features/files/     tree store, virtual TreeView, quick open; editor/: lazy CodeMirror
  features/theme/     VS Code -> --af-* mapping, overrides, engine; i18n/: typed t(), reportError()
  features/settings/  settings store (debounced save) + lazy SettingsPanel, sections (About)
  features/tokens/    usage store (one Channel), badge, chart, Usage view; updates/: update check
  features/agents/    lazy Agents library panel (wizard/, library/ Browse tab, installed), profiles
  features/workstreams/ protocol, engine, store, runtime, cards, sidebar section, lazy Timeline
  features/stage/     lazy Agent Stage: layout, positions, scene, motion, canvas renderer
  shared/             ipc (+ ipcCommands), types, errors, toast, dialog, markdown, linkSites, ui/
  locales/            en.json (source of truth), tr, de, zh, ru
themes/               built-in theme JSON (embedded into the Rust binary via include_str!)
```

Layering: `shared/` imports nothing from `features/` or `app/`. Core features (`i18n`, `settings`,
`theme`, `characters`, `agentui`, `providers`, `tokens`, `updates`) never import panel features.
Panel features (`terminal`, `files`, `editor`, `chat`) import core features and `workspace`, never
each other. `workstreams` and `stage` import core only (the Stage gets `StageHooks`); `layout`
and the settings panel compose panel features; `app/` composes all.

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

- One template (`agents/template.rs`, schema 1 = docs/agent-template.md + `schemas/`, a test
  keeps them in step) -> `targets.rs` renders exact files per tool (docs/agent-targets.md).
  Placeholders are plain substitution; unknown ones are errors; `\{{` is a literal `{{`.
- **Shared files** (CLAUDE.md, AGENTS.md, GEMINI.md) only get a section between
  `<!-- agentflow:begin <id> -->` / `end` markers (`section.rs`): nothing else touched, CRLF
  kept, idempotent. Every install is planned and previewed (`plan.rs`); foreign or changed
  files (hash in `.agentflow/installed.json`) and user scope (`roots.rs`) need a confirmation.
  Profiles narrow tools, never loosen the mode. New tool format = a target fn + snapshot test.
- Kinds: agent, rule, `skill` (folder with SKILL.md + `files`, binary as base64, scripts
  copied never run), `command` (Gemini). Library templates carry `source` (attribution).

## Community library (phase 10)

- `registry/sources.json` (docs/library-sources.md): GitHub main (ETag), else cache, else
  bundled; settings sources are `unverified`. Tree only on a new commit; files by commit, cached
  by blob sha; rate limit honoured (no request until the reset); auto refresh once a day.
- Importers (`library/import/`) keep the original text and list what they cannot convert
  (`Unsupported` + reason id); per-skill LICENSE files decide licenses. Safety flags (warnings
  only) live in `library/safety.rs` + tests. Browse hands templates to the wizard.

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
  One format reminder before a bad-reply pause (`autoRemind`). Only to members (`canRoute`).
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
  of 2-6 frames, `.` = transparent, else a base-36 palette index (`validate.ts`); names are
  never translated. ONE animation loop (`animator.ts`), only while visible; `reducedMotion()`.
  Eased movement snapped to sprite pixels (`motion.ts`); CSS breathing (scale >= 2) and hops.
- Terminal (`stateMachine.ts`): prompt -> think, output -> work, 2 s quiet -> done, 5 s -> idle,
  exit 0 -> sleep else error. Chat: send -> think, delta/tool -> work (bubble = tool status).

## Themes, i18n, settings

- VS Code `colors` -> `--af-*` (`theme/mapping.ts`), `tokenColors` -> `--af-syntax-*`; never
  hard-code colors. Overrides (text, accent, agent bold `#F5C542`, `--af-glow`): `overrides.ts`.
- Flat dotted keys in `src/locales/en.json`; `I18nKey = keyof typeof en`; every visible string
  uses `t('key', { param })`. New keys go into **all five** locales (typecheck + tests check).
- Rust owns `settings.json` (v11; `updates { checkOnStartup }`): bump `CURRENT_VERSION`
  and add a `migrate.rs` step on shape changes; `normalized()` repairs values; `defaults.test.ts`
  checks TS defaults against Rust.

## Design system (phase 11)

- Tokens (`shared/ui/tokens.css`): `--af-space-*` (4-based), `--af-text-*`, radii, control
  heights, elevation 1-3, `--af-ease` + 120/160/220 ms (0 with reduced motion), focus ring,
  accent glow. Colors only from theme tokens; gold (`--af-glow`) = highlights and "needs you".
- Shared components (`shared/ui/`): Button/IconButton, Input, Textarea, Select, Checkbox,
  Toggle, SegmentedControl, Tabs, Badge, Tooltip, Menu, Dialog (focus trap + restore), Toast,
  Card, EmptyState, Skeleton. **No one-off control styles**: new UI uses these (row-like list
  buttons share one rule in `controls.css`); icon-only buttons need a label.
- Icons, bundled, no network: Lucide (ISC, `shared/ui/lucide.ts` subset + `Icon`), Material
  Icon Theme file icons (MIT, `shared/ui/file-icons/`, `fileIconName()`), Simple Icons brands
  for links (CC0, `shared/linkSiteData.ts`, `siteForUrl()`; unknown = generic link). Licenses in
  `src/assets/icons/` and `file-icons/`. Never name a component like a Solid built-in
  (`Switch`, `Show`, `For`...): the JSX compiler resolves those to solid-js.

## CI, releases & updater (phase 12)

- CI (`.github/workflows/ci.yml`): Linux runs every check below, Windows `pnpm test` +
  `cargo test`, macOS `cargo test`. Keep it green; new checks go into all three if relevant.
- Release (`release.yml`, on `v*` tags): draft GitHub release with notes from CHANGELOG.md,
  installers for Windows (NSIS + MSI), macOS (aarch64 + x64 dmg), Linux (deb, AppImage, rpm)
  and the signed `latest.json`. Steps: `pnpm release:version x.y.z` -> CHANGELOG section ->
  commit + merge -> `git tag vx.y.z` + push -> check and publish the draft by hand.
- **Updater key:** only the PUBLIC key is in `tauri.conf.json`. The private key and its
  password live outside the repo and in the secrets `TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)`;
  never commit, log or print them. Signing (`APPLE_*`, `WINDOWS_CERTIFICATE*`) is optional.
- Updates never install silently: a check offers, only "Install and restart" installs. Panics
  unwind and go to `crash.log`. `pnpm notices` after dependency/asset changes (About > Licenses).

## Commands

```sh
pnpm install && pnpm tauri dev
pnpm typecheck && pnpm test && pnpm build && pnpm format:check
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
pnpm notices; pnpm release:version x.y.z   # notices file; version in all four files
```

## Roadmap (12 phases)

Done: all 12 phases (shell, terminals, files + editor, characters, provider chat, usage, API
agents, installer, workstreams + Stage, community library, UI polish + first-run, CI +
releases + updater). New work: an issue first, small PRs, no features beyond what was agreed.

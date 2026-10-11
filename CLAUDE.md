# AgentFlow

Open-source (MIT) desktop app for vibe coders: a multi-agent terminal and AI manager with a
VS Code-style dockable layout and cute pixel-art agent characters. Targets Windows, macOS and
Linux. Priorities: **low RAM, fast, clean code organization, fun look.**

## Stack (fixed)

- **Backend:** Tauri 2, Rust stable, tokio, serde/serde_json, thiserror, tracing,
  portable-pty (terminals), which (PATH lookup), ignore + notify (file tree, quick open,
  watcher), trash (deletes go to the OS trash), tauri plugins: dialog, clipboard-manager
- **Frontend:** SolidJS + TypeScript (strict) + Vite, pnpm, xterm.js (+ fit, webgl, unicode11),
  CodeMirror 6 (lazy; languages load per file type)
- **Layout:** `dockview-core` behind a thin Solid wrapper; its stylesheet comes from the `dockview`
  package (a pure re-export). Keep both pinned to the same version.
- **Styling:** plain CSS with `--af-*` variables; no React, Electron, Tailwind or UI kits. Bundled
  offline fonts: JetBrains Mono (code), Pixelify Sans (small UI accents only).

## Folder map

```
src-tauri/src/
  main.rs, lib.rs     app setup, managed state, command registration; error.rs: AppError
  settings/           versioned settings JSON in the OS app config dir (atomic write, migrations)
  themes/             built-in + imported VS Code themes: JSONC, validation, syntax colors, storage
  commands/           thin #[tauri::command] fns that delegate to modules
  pty/                PtyManager, sessions (reader/waiter/pump threads), shells, CLI detection
  fs/                 workspace: path checks, list/read/write/trash, quick open index, watcher
  prompts/            prompt history per workspace (app data dir, own JSON file per workspace)
  util/               atomic_write, run_blocking, lock/Signal, hex color check, test_dir
src/
  main.tsx, App.tsx   entry: bootstrap (settings -> locale -> theme -> workspace fs), then render
  app/                shell: ActivityBar, Sidebar (+ views), StatusBar, Toasts, Dialogs, styles/
  features/layout/    dockview host, panels, roles, placement, saved layout, tabs, characters
  features/terminal/  TerminalController, lazy TerminalPanel, agent/ overlay, prompt/ bar
  features/characters/  sprites/*.json, renderer, shared rAF animator, Avatar, state heuristic
  features/workspace/ workspace folder, fs session (backend open + watcher events), path helpers
  features/files/     Files panel: tree model/store, virtual TreeView, icons, quick open
  features/editor/    lazy EditorPanel + EditorController (CodeMirror), languages, dirty store
  features/theme/     VS Code -> --af-* mapping (+ syntax vars), user overrides, engine, store
  features/i18n/      typed t(), lazy locale loading, reportError()
  features/settings/  settings store (debounced save) + lazy SettingsPanel and sections
  features/{providers,tokens,agents}/, src-tauri/src/{providers,tokens,agents}/: empty stubs
  shared/             ipc, types, errors, toast, dialog, platform, dnd, debounce, color, ui/
  locales/            en.json (source of truth), tr, de, zh, ru
themes/               built-in theme JSON (embedded into the Rust binary via include_str!)
```

Layering: `shared/` imports nothing from `features/` or `app/`. Core features (`i18n`,
`settings`, `theme`, `characters`) never import panel features. Panel features (`terminal`,
`files`, `editor`) import core features and `workspace`, never each other. `layout` and the
settings panel compose panel features; `app/` composes all.

## Code rules

- Small files, one responsibility each; nothing over ~300 lines. No `any` in TS, no
  `unwrap()`/`expect()` in non-test Rust. Errors flow as `AppError` -> `reportError()` toast.
- All IPC goes through `src/shared/ipc.ts`; components never call `invoke` directly.
- Minimal dependencies; justify every new crate/package in the commit message.
- Performance: no polling (events, ResizeObserver, debounced saves), fine-grained signals
  (big blobs like the layout or tree listings stay out of reactive stores), lazy-load heavy
  panels (settings, terminal/xterm, editor/CodeMirror, non-English locales).
- Rust file I/O in commands runs through `util::run_blocking`, never on the main thread.

## IPC pattern

1. Rust: a fn in the owning module, then a thin `#[tauri::command] async fn` in
   `commands/<area>.rs` returning `AppResult<T>`, registered in `lib.rs`.
2. TS: an entry in the `Commands` map of `shared/ipc.ts` plus a wrapper on `ipc`; mirror Rust
   structs in `shared/types.ts` (serde camelCase). Errors arrive as `AppError { kind, message }`
   (`tooLarge` also has `size`); `toAppError()` normalizes anything thrown.

## PTY / Channel pattern

- No global events: `ipc.ptySpawn()` creates a `Channel` _before_ `pty_spawn`; Rust sends raw
  bytes (coalesced ~10 ms / 64 KB, decoded by the frontend's `OutputDecoder`) and a final JSON
  `{ type: 'exit', code, signal }`, then drops the channel (unregistering the JS callback).
- Kill (panel close, `pty_kill`, exit, reload) = SIGHUP / TerminateProcess, SIGKILL after 2 s;
  the waiter thread reaps. Flow control: `pty_ack` after xterm parses, reader pauses at 512 KB.
- Persist terminal _config_ only (incl. `character` and `historyKey`). Restored agent CLIs never
  auto-start; shells follow `terminal.autoStartShells`.

## FS / watcher pattern

- Frontend paths are workspace-relative with `/`; `fs/paths.rs` canonicalizes and rejects `..`,
  absolute paths and escaping symlinks. One `ignore` config (`fs/walk.rs`: .gitignore even
  outside git, `.git` always hidden, dotfiles per `files.showHidden`) for list, quick open, watcher.
- `fs_open_workspace(root, showHidden, Channel)`: notify with a 200 ms debounce; batches
  `{ paths, structural, rescan }` go to `workspace/fsSession.ts`, which fans them out.
- Saves are atomic with the mtime the file was read at (mismatch = `conflict`: Reload / Keep
  mine). Deletes go to the OS trash. Reads stop at 5 MB.

## Layout roles

- Groups have a role (`layout/roles.ts`): `agents` (center: agent CLIs, editors, settings),
  `files` (right, ~22%), `terminal` (shells, ~30% under the center). New panels open in the last
  focused group of their role; `placement.ts` recreates a missing one at its default spot.
- Saved layout = `{ version: 2, dock, roles }`, else the default layout. Bump `LAYOUT_VERSION`
  in `savedLayout.ts` when panel shapes change incompatibly.

## Characters & prompt bar

- Sprites: `characters/sprites/*.json` = `{ id, name, size, palettes, frameMs, states }`, size 16
  (32 accepted), 2-3 palettes of hex colors, 8 states (idle walk think work receive done error
  sleep) of 2-4 frames; a frame is `size` strings of `size` chars, `.` = transparent, else a
  base-36 palette index (`validate.ts`). Character names are never translated.
- `renderer.ts` caches one ImageBitmap strip per (character, palette, integer scale, state). All
  motion uses the ONE loop in `animator.ts`, which only runs while something animates on screen
  in a visible document; idle/sleep are still frames (~0 CPU at rest). Respect `reducedMotion()`.
- State heuristic (`stateMachine.ts`; `terminal/agentActivity.ts`, one timer per panel): prompt
  -> think; output -> work; 2 s quiet -> done; 5 s later -> idle; exit 0 -> sleep, else error.
  Output within 400 ms of a prompt or typed keys is their echo, not work.
- Agent panels (shells only with `characters.onShells`) get an unused character on open
  (`layout/characters.ts`). The prompt bar writes to the pty first (`encodePrompt`: bracketed
  paste for multiline text when enabled, then CR); animations are decoration only.

## Themes, i18n, settings

- VS Code theme format: `colors` map to `--af-*` in `theme/mapping.ts` (keys, then a `ref` or
  dark/light fallback); never hard-code colors. `tokenColors` -> `agentflow.syntax.*` (Rust) ->
  `--af-syntax-*`, else the ANSI palette. Overrides (text, accent, agent bold `--af-bold-color`
  `#F5C542`, `--af-glow`) live in `overrides.ts`; imports are `user:<slug>` themes on disk.
- Flat dotted keys in `src/locales/en.json`; `I18nKey = keyof typeof en`; every visible string
  uses `t('key', { param })`. New keys go into **all five** locales (typecheck + tests check).
- Rust owns `settings.json` (v4): bump `CURRENT_VERSION` + add a `migrate.rs` step when the shape
  changes; `normalized()` repairs values. The frontend uses `updateSettings()` (300 ms debounce).

## Commands

```sh
pnpm install && pnpm tauri dev
pnpm typecheck && pnpm test && pnpm build
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cargo build    # in src-tauri: compile check, no bundling
```

## Roadmap (9 phases)

1. [x] Skeleton, theme engine, i18n, dockable layout
2. [x] PTY + multi terminal panels (xterm.js WebGL, portable-pty)
3. [x] Workspace file tree (notify + ignore) + CodeMirror 6 viewer/editor
4. [x] Pixel character system (15 characters, 16x16 JSON sprites scaled, typewriter speech bar)
5. [ ] AI providers (CLI agents + Anthropic/OpenAI/Gemini/DeepSeek/compatible APIs, keyring keys)
6. [ ] Token & cost tracking
7. [ ] Agent installer wizard (per-AI formats/paths)
8. [ ] Community agent library from GitHub
9. [ ] Cross-platform builds + CI

Only build what the current phase lists; leave future modules as empty stubs.

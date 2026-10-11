# AgentFlow

Open-source (MIT) desktop app for vibe coders: a multi-agent terminal and AI manager with a
VS Code-style dockable layout and cute pixel-art agent characters. Targets Windows, macOS and
Linux. Priorities: **low RAM, fast, clean code organization, fun look.**

## Stack (fixed)

- **Backend:** Tauri 2, Rust stable, tokio, serde/serde_json, thiserror, tracing,
  portable-pty (terminals), which (PATH lookup), tauri-plugin-dialog (folder picker)
- **Frontend:** SolidJS + TypeScript (strict) + Vite, pnpm, xterm.js (+ fit, webgl, unicode11)
- **Layout:** `dockview-core` (vanilla) behind a thin Solid wrapper. Its stylesheet ships in the
  `dockview` package (a pure re-export of dockview-core); keep both pinned to the same version.
- **Styling:** plain CSS with `--af-*` CSS variables. No React, Electron, Tailwind or UI kits.
- **Fonts:** bundled offline: JetBrains Mono (code), Pixelify Sans (small UI accents only).

## Folder map

```
src-tauri/src/
  main.rs, lib.rs     app setup, managed state, command registration
  error.rs            AppError (thiserror) -> serialized { kind, message } for the frontend
  settings/           versioned settings JSON in the OS app config dir (atomic write, migrations)
  themes/             built-in + imported VS Code themes: JSONC parsing, validation, storage
  commands/           thin #[tauri::command] fns that delegate to modules
  pty/                PtyManager, sessions (reader/waiter/pump threads), shells, CLI detection
  util/               atomic_write, run_blocking, lock/Signal, hex color check, test_dir
  fs/ providers/ tokens/ agents/   future phases (empty)
src/
  main.tsx, App.tsx   entry: bootstrap (settings -> locale -> theme), then render
  app/                shell: ActivityBar, Sidebar (+ views), StatusBar, Toasts, styles/
  features/layout/    dockview host, panel registry, renderers, tabs, layout persistence
  features/terminal/  TerminalController (xterm + pty session), lazy TerminalPanel, stores
  features/workspace/ current/recent workspace folder, folder picker
  features/theme/     VS Code -> --af-* mapping, user overrides, engine, theme store
  features/i18n/      typed t(), lazy locale loading, reportError()
  features/settings/  settings store (debounced save) + lazy SettingsPanel and sections
  features/{files,characters,providers,tokens,agents}/   future (empty)
  shared/             ipc.ts, types.ts, errors.ts, toast.ts, debounce.ts, color.ts, ui/
  assets/fonts/       woff2 + fonts.css + OFL license files
  locales/            en.json (source of truth), tr, de, zh, ru
themes/               built-in theme JSON (embedded into the Rust binary via include_str!)
```

Layering: `shared/` imports nothing from `features/` or `app/`. Core features (`i18n`,
`settings`, `theme`) never import panel features. Panel features (`terminal`, `workspace`) may
import core features. `layout` and the settings panel compose panel features; `app/` composes all.

## Code rules

- Small files, one responsibility each; nothing over ~300 lines.
- No `any` in TS. No `unwrap()`/`expect()` in non-test Rust code. Errors flow as `AppError`
  to the frontend and are shown as a toast via `reportError(contextKey, err)`.
- All IPC goes through `src/shared/ipc.ts`; components never call `invoke` directly.
- Minimal dependencies; justify every new crate/package in the commit message.
- Performance: no polling (events, ResizeObserver, debounced saves), fine-grained signals
  (big blobs like the layout stay out of reactive stores), lazy-load heavy panels (settings,
  terminal/xterm, non-English locales).
- Rust file I/O in commands runs through `util::run_blocking`, never on the main thread.

## IPC pattern

1. Rust: add a fn in the owning module, then a thin `#[tauri::command] async fn` in
   `commands/<area>.rs` returning `AppResult<T>`; register it in `lib.rs`.
2. TS: add the command to the `Commands` map in `shared/ipc.ts` (args + result types) and a
   wrapper on the `ipc` object. Mirror Rust structs in `shared/types.ts` (serde camelCase).
3. Errors arrive as `AppError { kind, message }`; `toAppError()` normalizes anything thrown.

## PTY / Channel pattern

- Output never uses global events. `ipc.ptySpawn()` creates a `Channel` _before_ invoking
  `pty_spawn`; Rust sends coalesced raw bytes (`InvokeResponseBody::Raw`, an `ArrayBuffer` in
  JS, flushed after ~10 ms or 64 KB) and one final JSON `{ type: 'exit', code, signal }`.
- Rust never decodes or splits UTF-8; the frontend's streaming `OutputDecoder` does.
- When the session ends Rust drops the channel, which unregisters the JS callback. Kill on panel
  close / `pty_kill` / app exit = SIGHUP (TerminateProcess on Windows), SIGKILL after 2 s; the
  waiter thread always reaps the child, and the session removes itself from the map.
- Input and resizes go through per-session queues (`queues.ts`) so IPC calls stay in order.
- Persist terminal _config_ only (kind, command, args, cwd). Restored agent CLIs never
  auto-start; restored shells follow `terminal.autoStartShells`.

## Theme conventions

- VS Code color theme format; only `colors` is used (`tokenColors` kept for the editor phase).
- `features/theme/mapping.ts` lists every `--af-*` variable (VS Code keys, then a `ref` or a
  dark/light fallback). Add UI colors there; never hard-code colors in components.
- Overrides (text, accent, agent bold: `--af-bold-color` default `#F5C542`, `--af-glow`) live in
  `overrides.ts`. Built-in themes in `themes/` must pass `cargo test`; imports are saved as
  `<app config>/themes/<slug>.json` with id `user:<slug>`.

## i18n conventions

- Flat dotted keys in `src/locales/en.json`; `I18nKey = keyof typeof en`. Every visible string
  goes through `t('key', { param })` (brand name and language endonyms excepted).
- New keys go into **all five** locales: a missing key fails `pnpm typecheck`, and `pnpm test`
  checks keys and `{placeholders}`.

## Settings

Rust owns `settings.json`: bump `CURRENT_VERSION` and add a `migrate.rs` step when the shape
changes; `normalized()` repairs bad values; a corrupt file goes to `settings.json.corrupt`.
The frontend applies changes via `updateSettings()` and saves with a 300 ms debounce.

## Commands

```sh
pnpm install && pnpm tauri dev
pnpm typecheck && pnpm test && pnpm build
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cargo build    # in src-tauri: compile check without bundling
```

## Roadmap (9 phases)

1. [x] Skeleton, theme engine, i18n, dockable layout (done)
2. [x] PTY + multi terminal panels (xterm.js WebGL, portable-pty) (done)
3. [ ] Workspace file tree (notify + ignore) + CodeMirror 6 viewer/editor
4. [ ] Pixel character system (15 characters, 16x16 JSON sprites scaled, typewriter speech bar)
5. [ ] AI providers (CLI agents + Anthropic/OpenAI/Gemini/DeepSeek/compatible APIs, keyring keys)
6. [ ] Token & cost tracking
7. [ ] Agent installer wizard (per-AI formats/paths)
8. [ ] Community agent library from GitHub
9. [ ] Cross-platform builds + CI

Only build what the current phase lists; leave future modules as empty stubs.

# AgentFlow

Open-source (MIT) desktop app for vibe coders: a multi-agent terminal and AI manager with a
VS Code-style dockable layout and cute pixel-art agent characters. Targets Windows, macOS and
Linux. Priorities: **low RAM, fast, clean code organization, fun look.**

## Stack (fixed)

- **Backend:** Tauri 2, Rust stable, tokio, serde/serde_json, thiserror, tracing
- **Frontend:** SolidJS + TypeScript (strict) + Vite, pnpm
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
  util/               atomic_write, run_blocking, hex color check, test_dir (tests only)
  pty/ fs/ providers/ tokens/ agents/   future phases (empty)
src/
  main.tsx, App.tsx   entry: bootstrap (settings -> locale -> theme), then render
  app/                shell: ActivityBar, Sidebar (+ views), StatusBar, Toasts, styles/
  features/layout/    dockview host, panel registry, renderers, layout persistence
  features/theme/     VS Code -> --af-* mapping, user overrides, engine, theme store
  features/i18n/      typed t(), lazy locale loading, reportError()
  features/settings/  settings store (debounced save) + lazy SettingsPanel and sections
  features/{terminal,files,characters,providers,tokens,agents}/   future (empty)
  shared/             ipc.ts, types.ts, errors.ts, toast.ts, debounce.ts, color.ts, ui/
  assets/fonts/       woff2 + fonts.css + OFL license files
  locales/            en.json (source of truth), tr, de, zh, ru
themes/               built-in theme JSON (embedded into the Rust binary via include_str!)
```

Layering: `shared/` imports nothing from `features/` or `app/`. Features may import `shared/`,
`features/i18n` and `features/settings`. `app/` composes features.

## Code rules

- Small files, one responsibility each; nothing over ~300 lines.
- No `any` in TS. No `unwrap()`/`expect()` in non-test Rust code. Errors flow as `AppError`
  to the frontend and are shown as a toast via `reportError(contextKey, err)`.
- All IPC goes through `src/shared/ipc.ts`; components never call `invoke` directly.
- Minimal dependencies; justify every new crate/package in the commit message.
- Performance: no polling (events, ResizeObserver, debounced saves), no heavy re-renders
  (fine-grained signals, large blobs like the layout kept out of reactive stores), lazy-load
  panels that are not visible at startup (settings panel, non-English locales).
- Rust file I/O in commands runs through `util::run_blocking`, never on the main thread.

## IPC pattern

1. Rust: add a fn in the owning module, then a thin `#[tauri::command] async fn` in
   `commands/<area>.rs` returning `AppResult<T>`; register it in `lib.rs`.
2. TS: add the command to the `Commands` map in `shared/ipc.ts` (args + result types) and a
   wrapper on the `ipc` object. Mirror Rust structs in `shared/types.ts` (serde camelCase).
3. Errors arrive as `AppError { kind, message }`; `toAppError()` normalizes anything thrown.

## Theme conventions

- Themes use the VS Code color theme format. Only `colors` is used; `tokenColors` is stored
  on import and ignored until the editor phase.
- `features/theme/mapping.ts` lists every `--af-*` variable: VS Code keys tried in order, then a
  fallback (`ref` to an earlier variable, or a dark/light default). Add new UI colors there
  and use `var(--af-…)` in CSS; never hard-code colors in components.
- User overrides (text, accent, agent bold) are layered in `overrides.ts`. `.af-agent-bold`
  uses `--af-bold-color` (default `#F5C542`) and `--af-glow`.
- Built-in themes live in `themes/` and must pass `cargo test` (validation + core keys).
- Imported themes are saved to `<app config>/themes/<slug>.json` with id `user:<slug>`.

## i18n conventions

- Flat dotted keys in `src/locales/en.json`; `I18nKey = keyof typeof en`.
- Every visible string goes through `t('key', { param })`; no literals in components
  (brand name and language endonyms excepted).
- Adding a key: add it to **all five** locale files. A missing key fails `pnpm typecheck`
  (loaders are typed against `Dictionary`) and `pnpm test` checks keys and `{placeholders}`.

## Settings

- Rust owns `settings.json` (`version` field + `migrate.rs` steps; bump `CURRENT_VERSION` and
  add a step when the shape changes). Unknown/invalid values are repaired by `normalized()`;
  a corrupt file is moved to `settings.json.corrupt` and defaults are used.
- Frontend changes apply instantly via `updateSettings()` and are saved with a 300 ms debounce.

## Commands

```sh
pnpm install && pnpm tauri dev
pnpm typecheck && pnpm test && pnpm build
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
pnpm tauri build --debug
```

## Roadmap (9 phases)

1. [x] Skeleton, theme engine, i18n, dockable layout (done)
2. [ ] PTY + multi terminal panels (xterm.js WebGL, portable-pty)
3. [ ] Workspace file tree (notify + ignore) + CodeMirror 6 viewer/editor
4. [ ] Pixel character system (15 characters, 16x16 JSON sprites scaled, typewriter speech bar)
5. [ ] AI providers (Claude Code/Codex/Gemini CLI + Anthropic/OpenAI/Gemini/DeepSeek/
       OpenAI-compatible APIs, keys in OS keyring)
6. [ ] Token & cost tracking
7. [ ] Agent installer wizard (per-AI formats/paths)
8. [ ] Community agent library from GitHub
9. [ ] Cross-platform builds + CI

Only build what the current phase lists; leave future modules as empty stubs.

# Contributing to AgentFlow

Thanks for helping! Bug reports, ideas and pull requests are all welcome. For anything bigger
than a small fix, please open an issue first so we can agree on the approach.

## Development setup

You need Rust (stable, via [rustup](https://rustup.rs)), Node.js 22+ and pnpm 10+, plus the
Tauri system dependencies for your OS. The [README](README.md#development-setup) lists them
per platform.

```sh
pnpm install
pnpm tauri dev          # the desktop app with hot reload
```

Debug builds keep settings and data in `AGENTFLOW_DATA_DIR` when it is set, so you can try
things without touching your real setup.

## Checks

CI runs these on every pull request (Linux runs everything; Windows and macOS run the tests).
Please run them before you push:

```sh
pnpm typecheck && pnpm test && pnpm build && pnpm format:check
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Code rules (short version of [CLAUDE.md](CLAUDE.md))

- Small files with one job each (about 300 lines at most). TypeScript is strict: no `any`.
  Rust: no `unwrap()`/`expect()` outside tests; errors are `AppError` and reach the user as a
  translated toast.
- All IPC goes through `src/shared/ipc.ts`: a Rust fn in its module, a thin
  `#[tauri::command]` in `src-tauri/src/commands/`, an entry in `shared/ipcCommands.ts`.
- No polling, lazy-load heavy panels, keep big data out of reactive stores. Low RAM matters.
- UI: use the shared components in `src/shared/ui/` and the `--af-*` theme variables; no
  one-off control styles and no hard-coded colors.
- Every visible string uses `t('key')`, and new keys go into **all five** locales
  (`src/locales/`: en, tr, de, zh, ru).
- API keys only ever live in the OS keychain. Agents never answer permission prompts on their
  own, and dangerous commands always ask.
- Settings shape changes: bump `CURRENT_VERSION` and add a `migrate.rs` step with a test.
- Keep dependencies minimal and say why a new one is needed in the commit message.
- Write tests for new or changed code.

## Commits and pull requests

- Small, logical commits in the [Conventional Commits](https://www.conventionalcommits.org)
  style: `feat(chat): ...`, `fix(pty): ...`, `docs: ...`, `test: ...`, `ci: ...`. Explain the
  why in the body when it is not obvious.
- Pull requests use the template's checklist; screenshots for UI changes (demo content only,
  no personal data or keys).
- If you change dependencies or bundled assets, run `pnpm notices` and commit the updated
  `THIRD_PARTY_NOTICES.md`.

## Releasing (maintainers)

1. `pnpm release:version 1.2.3` sets the version in package.json, Cargo.toml, Cargo.lock and
   tauri.conf.json together.
2. Add a `## [1.2.3] - YYYY-MM-DD` section to `CHANGELOG.md` (user-facing notes) and update the
   compare links at the bottom; run `pnpm notices` if dependencies changed.
3. Commit (`chore: release 1.2.3`) and merge to `main`. Then tag that commit and push the tag:
   `git tag v1.2.3`, `git push origin v1.2.3`. A `-rc.N` suffix (`v1.2.3-rc.1`) makes a
   pre-release for testing.
4. The Release workflow builds every installer and the updater's `latest.json` into a **draft**
   release. Check the files, then publish it on GitHub. Installed apps see the update once it
   is published (pre-releases and drafts are never offered).

The updater signs with a private key that is **never** committed. The workflow reads it from
the repository secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`;
the public key is in `src-tauri/tauri.conf.json`. Losing the private key means existing
installs can no longer verify updates, so keep a backup in a password manager.

Code signing is optional and only used when its secrets exist:

- macOS: `APPLE_CERTIFICATE` (base64 .p12), `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_SIGNING_IDENTITY`; notarization also needs `APPLE_ID`, `APPLE_PASSWORD` (an
  app-specific password) and `APPLE_TEAM_ID`.
- Windows: `WINDOWS_CERTIFICATE` (base64 .pfx) and `WINDOWS_CERTIFICATE_PASSWORD`.

## What and why

<!-- What does this change, and why? Link the issue: "Fixes #123". -->

## Checklist

- [ ] `pnpm typecheck`, `pnpm test`, `pnpm build`, `pnpm format:check` pass
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` pass (in `src-tauri`)
- [ ] Tests for new or changed code
- [ ] New visible strings use `t()` and are in all five locales
- [ ] UI uses the shared components and `--af-*` variables (screenshots below, demo content only)
- [ ] Settings shape changed? `CURRENT_VERSION` bumped with a `migrate.rs` step
- [ ] Dependencies or bundled assets changed? Justified in the commit, `pnpm notices` re-run
- [ ] No keys, tokens or personal data in code, logs, screenshots or tests

## Screenshots

<!-- For UI changes. -->

# Repository Guidelines

## Project Structure & Modules
- `src/` Rust sources: `cli/` (argument parsing, handlers), `core/` (switcher, session, constants), `environments/` (java/llm/cc managers), `infrastructure/` (config, shell scripts, network), `utils/` (fs/env helpers).
- `scripts/` Node helpers for install/build; `bin/` JS entry for npm users.
- `config/` example configs; `target/` build outputs (ignored in VCS).

## Build, Test, Develop
- `cargo build --release` — build native binary (`target/release/fnva[.exe]`).
- `cargo test` — run Rust test suite.
- `npm run build` — package cross-platform binaries via scripts (uses `scripts/build-*.sh`).
- `npm run check-permissions` — validate shell integration permissions.
Use `Set-Location <repo>` / `cd <repo>` before running; on Windows PowerShell prefer `;` as separator.

## Coding Style & Naming
- Rust: follow `rustfmt` (`.rustfmt.toml`) and `clippy` (`.clippy.toml`). Run `cargo fmt` then `cargo clippy --all-targets -- -D warnings` before PRs.
- Naming: modules/files `snake_case`, types/traits `PascalCase`, functions/vars `snake_case`, constants `SCREAMING_SNAKE_CASE`.
- Comments: concise, only to clarify non-obvious logic (e.g., environment resolution order).

## Testing Guidelines
- Framework: built-in Rust tests (`cargo test`).
- Add targeted unit tests for new logic (e.g., environment managers, script generation). Name tests after the behavior: `test_switch_records_session`, `test_parse_shell_type`.
- For shell/template changes, prefer small snapshot-like string assertions over integration shelling-out unless necessary.

## Commit & Pull Request Guidelines
- Commits: imperative, concise, scoped (e.g., `Persist llm add to config`, `Fix powershell current detection`). Squash small fixups when possible.
- PRs should describe: intent, key changes, testing done (`cargo test`, `npm run build` if relevant), and any risk areas (config writes, shell scripts). Link issue IDs if available.

## Security & Configuration Tips
- User config lives in `~/.fnva/config.toml` (Windows: `%USERPROFILE%\.fnva\config.toml`); avoid committing real API keys. Support env var placeholders like `${OPENAI_API_KEY}`.
- Shell integration scripts are generated; do not hardcode secrets or absolute user paths in source. Prefer template values and environment variables.

## Chat
- Respond in chinese by default

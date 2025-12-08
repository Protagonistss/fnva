# fnva - Fast Environment Version Alter

[中文](README_zh.md) · [Docs](docs/) · [Releases](https://github.com/Protagonistss/fnva/releases)

fnva is a cross-platform environment switcher for Java, Claude Code (CC), and general LLM setups. It is written in Rust, starts instantly, and works via shell snippets without background daemons.

## Features

- Manage multiple Java/CC/LLM profiles with per-shell activation and global defaults.
- Auto-generate shell init scripts for PowerShell, Bash, Zsh, Fish, CMD.
- Scan and dedupe local JDKs, switch by name in one command.
- Configure CC endpoints and LLM API keys in one place, export as env vars.
- Single static binary; no runtime dependencies.

## Installation

```bash
# npm (recommended for quick start)
npm install -g fnva

# Cargo
cargo install fnva
```

Or grab a platform binary from [Releases](https://github.com/Protagonistss/fnva/releases) and put it on your `PATH`.

## Shell integration

Enable auto-loading on shell startup:

- PowerShell:
  ```powershell
  fnva env env --shell powershell | Out-String | Invoke-Expression
  ```
- Bash:
  ```bash
  eval "$(fnva env env --shell bash)"
  ```
- Zsh:
  ```bash
  eval "$(fnva env env --shell zsh)"
  ```
- Fish:
  ```fish
  fnva env env --shell fish | source
  ```

## Usage

### Java
- Scan: `fnva java scan`
- List: `fnva java list`
- Use (session): `eval "$(fnva java use jdk-17)"`
- Set default: `fnva java default jdk-17`
- Add manual entry: `fnva java add --name jdk-8 --home "C:\\Java\\jdk1.8.0" --description "Legacy JDK"`

### Claude Code (CC)
- List: `fnva cc list`
- Add (GLM-4 example):
  ```bash
  fnva cc add glmcc '{
    "provider": "anthropic",
    "api_key": "your-api-key",
    "base_url": "https://open.bigmodel.cn/api/anthropic",
    "model": "glm-4.6",
    "description": "GLM-4"
  }'
  ```
- Use: `eval "$(fnva cc use glmcc)"`

### LLM
- Add: `fnva llm add --name openai-dev --provider openai --api-key "sk-..." --model gpt-4`
- Use: `eval "$(fnva llm use openai-dev)"`

## Configuration

- Location: `~/.fnva/config.toml` (Windows: `%USERPROFILE%\.fnva\config.toml`)
- Example:
  ```toml
  custom_java_scan_paths = ["D:\\Environment\\Java", "/opt/java"]

  [[java_environments]]
  name = "jdk-21"
  java_home = "C:\\Program Files\\Java\\jdk-21"
  description = "Oracle JDK 21"

  [[cc_environments]]
  name = "glmcc"
  provider = "anthropic"
  api_key = "sk-..."
  base_url = "https://open.bigmodel.cn/api/anthropic"
  model = "glm-4.6"
  ```

## Commands quick reference

| Command | Purpose |
| --- | --- |
| `fnva env env` | Generate shell init snippet |
| `fnva <type> list` | List environments (type: java/cc/llm) |
| `fnva <type> use <name>` | Emit snippet to activate an environment |
| `fnva <type> current` | Show current environment |
| `fnva <type> default <name>` | Get/set default (java/cc) |
| `fnva <type> remove <name>` | Remove an environment |
| `fnva java scan` | Scan local JDKs |
| `fnva config sync` | Sync/upgrade config schema |

## Build and release

- Format/lint: `cargo fmt && cargo clippy --all-targets -- -D warnings`
- Test: `cargo test`
- Build: `cargo build --release`
- Platform bundles: `npm run build:platforms`
- CI: tag `v*` to build binaries, publish to GitHub Releases, npm (`NPM_TOKEN`), and crates.io (`CARGO_TOKEN`).

## License

MIT License.

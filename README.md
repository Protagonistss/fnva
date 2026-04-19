<div align="center">

# fnva - Fast Environment Version Alter

**Fast, cross-platform environment version switcher**

[中文文档](README_zh.md) · [Architecture](docs/architecture/core-design.md) · [Releases](https://github.com/Protagonistss/fnva/releases)

[![npm version](https://img.shields.io/npm/v/fnva)](https://www.npmjs.com/package/fnva)
[![crates.io](https://img.shields.io/crates/v/fnva)](https://crates.io/crates/fnva)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

<!-- TODO: Add demo.gif here -->

</div>

fnva is a cross-platform environment switcher for Java, Claude Code (CC), and general LLM setups. Written in Rust, it starts instantly, has zero runtime dependencies, and works via shell snippets without background daemons.

## Core Features

- ⚡ **Fast & Zero Dependencies**: Single static binary.
- 🔄 **Session & Global Switching**: Per-shell activation and global defaults.
- 🐚 **Broad Shell Support**: PowerShell, Bash, Zsh, Fish, CMD.
- 🧠 **Auto-restore**: Opening a new terminal automatically restores your last active environment.
- ☕ **Smart Java Management**: Scan and dedupe local JDKs.
- 🤖 **Unified LLM Setup**: Configure LLM API keys in one place.

## Documentation Navigation

- [Core Architecture](docs/architecture/core-design.md) (Chinese)
- [Roadmap](docs/development/roadmap.md) (Chinese)
- [Contributing](docs/development/contributing.md) (Chinese)
- [Shell Integration Guide](docs/user-guide/shell-integration.md)

## Installation

```bash
# npm (recommended)
npm install -g fnva

# Cargo
cargo install fnva
```

Or download from [Releases](https://github.com/Protagonistss/fnva/releases) and add to `PATH`.

## Quick Start

- Shell Integration: `eval "$(fnva env env --shell bash)"`
- Scan Java: `fnva java scan`
- Switch Java: `fnva java use jdk-17` (with shell integration)
- Switch CC profile: `fnva cc use glmcc`

## Configuration
Config is stored at `~/.fnva/config.toml` (Windows: `%USERPROFILE%\.fnva\config.toml`).

## License
MIT License.

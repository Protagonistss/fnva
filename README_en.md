<div align="center">

# fnva - Fast Environment Version Alter

**Fast, cross-platform environment version switcher**

[中文文档](README_zh.md) · [Architecture](docs/architecture/core-design.md) · [Releases](https://github.com/Protagonistss/fnva/releases)

[![npm version](https://img.shields.io/npm/v/fnva)](https://www.npmjs.com/package/fnva)
[![crates.io](https://img.shields.io/crates/v/fnva)](https://crates.io/crates/fnva)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

<!-- TODO: Add demo.gif here -->

</div>

fnva is a cross-platform environment switcher for Java, Maven, and Claude Code (CC). Written in Rust, it starts instantly, has zero runtime dependencies, and works via shell snippets without background daemons.

## Core Features

- ⚡ **Fast & Zero Dependencies**: Single static binary.
- 🔄 **Session & Global Switching**: Per-shell activation and global defaults.
- 🐚 **Broad Shell Support**: PowerShell, Bash, Zsh, Fish, CMD.
- 🧠 **Auto-restore**: Opening a new terminal automatically restores your last active environment.
- ☕ **Smart Java Management**: Scan local JDKs, and automatically download/install version-specific JDKs via Adoptium mirror resolver.
- 📦 **Maven Version Management**: Query remote versions from mirror directory, auto-install, and switch `MAVEN_HOME`.
- 🔌 **Tool Source Protocol (TSP)**: Unified abstraction for tool discovery and downloading shared by Java and Maven modules.

## Documentation Navigation

- [Core Architecture](docs/architecture/core-design.md) (Chinese)
- [Roadmap](docs/development/roadmap.md) (Chinese)
- [Contributing](docs/development/contributing.md) (Chinese)
- [Tool Source Protocol Spec](docs/spec/tool-source-protocol.md)
- [Shell Integration Guide](docs/user-guide/shell-integration.md)
- [Terminal Encoding Fix Guide](docs/user-guide/encoding-fixes.md)

## Installation

```bash
# npm (recommended)
npm install -g fnva

# Cargo
cargo install fnva
```

Or download from [Releases](https://github.com/Protagonistss/fnva/releases) and add to `PATH`.

## Shell Integration

With shell integration, opening a new terminal restores your last active CC/Java/Maven environments automatically, and `fnva <type> use <name>` works directly without needing an `eval` wrapper. See [Shell Integration Guide](docs/user-guide/shell-integration.md) for details.

## Quick Start

### Java
- Scan local JDKs: `fnva java scan`
- List remote versions: `fnva java ls-remote`
- Install a version: `fnva java install 17`
- List local environments: `fnva java list`
- Switch version: `fnva java use 17` (or `eval "$(fnva java use 17)"` if shell integration is not installed)
- Set default version: `fnva java default 17`

### Maven
- List remote versions: `fnva maven ls-remote`
- Install a version: `fnva maven install 3.9.16`
- List local environments: `fnva maven list`
- Switch version: `fnva maven use 3.9.16` (or `eval "$(fnva maven use 3.9.16)"` if shell integration is not installed)
- Refresh remote cache: `fnva maven refresh`

### Claude Code (CC)
- List environments: `fnva cc list`
- Switch environment: `fnva cc use mycc` (or `eval "$(fnva cc use mycc)"` if shell integration is not installed)

## Configuration

User configuration is stored at `~/.fnva/config.toml` (Windows: `%USERPROFILE%\.fnva\config.toml`).

## License

MIT License.

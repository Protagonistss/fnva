# fnva - Fast Environment Version Alter

[![npm version](https://img.shields.io/npm/v/fnva)](https://www.npmjs.com/package/fnva) [![crates.io](https://img.shields.io/crates/v/fnva)](https://crates.io/crates/fnva) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

[中文文档](README_zh.md) · [Full English](README_en.md) · [Docs](docs/)

Cross-platform environment switcher for Java, Claude Code (CC), and LLM setups. Rust single binary with fast startup and zero runtime dependencies.

## Install

- npm: `npm install -g fnva`
- Cargo: `cargo install fnva`
- Binary: download from [Releases](https://github.com/Protagonistss/fnva/releases) and add to `PATH`.

## Quick start

- Init shell (Bash/Zsh): `eval "$(fnva env env --shell bash)"`  
  PowerShell: `fnva env env --shell powershell | Out-String | Invoke-Expression`
- Scan Java: `fnva java scan`
- Switch Java for current session: `eval "$(fnva java use jdk-17)"`
- Switch CC profile: `eval "$(fnva cc use glmcc)"`

## What it does

- Manages multiple Java, CC, and generic LLM configurations.
- Generates shell snippets to activate environments per session or by default.
- Stores config at `~/.fnva/config.toml` (Windows: `%USERPROFILE%\.fnva\config.toml`).
- Ships as a single binary; no background daemon.

## Build/test locally

```
cargo fmt && cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

For platform bundles: `npm run build:platforms`

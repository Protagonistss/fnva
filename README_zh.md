<div align="center">

# fnva - Fast Environment Version Alter

**极速、跨平台的命令行环境版本切换工具**

[English](README_en.md) · [架构文档](docs/architecture/core-design.md) · [Releases](https://github.com/Protagonistss/fnva/releases)

[![npm version](https://img.shields.io/npm/v/fnva)](https://www.npmjs.com/package/fnva)
[![crates.io](https://img.shields.io/crates/v/fnva)](https://crates.io/crates/fnva)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

<!-- TODO: Add demo.gif here -->

</div>

fnva 是一个面向 Java、Claude Code (CC) 和通用 LLM 场景的跨平台环境切换工具。基于 Rust 编写，具备**极速启动**、**零依赖**的特点，通过生成 shell 片段完成环境激活，完全**无常驻后台进程**。

## 核心特性

- ⚡ **极速与零依赖**: 单一静态二进制文件。
- 🔄 **会话与全局切换**: 支持当前终端会话级别的切换，也可设置全局默认环境。
- 🐚 **全平台 Shell 支持**: 原生支持 PowerShell、Bash、Zsh、Fish、CMD。
- 🧠 **自动恢复**: 打开新终端自动恢复上次使用的环境。
- ☕ **智能 Java 管理**: 扫描本地 JDK，一键切换。
- 🤖 **统一的大模型管理**: 集中配置 LLM API 密钥。

## 文档导航

- [架构设计与原理](docs/architecture/core-design.md)
- [开发路线图](docs/development/roadmap.md)
- [贡献指南](docs/development/contributing.md)
- [Shell 集成与自动恢复](docs/user-guide/shell-integration.md)
- [终端乱码修复指南](docs/user-guide/encoding-fixes.md)

## 安装

```bash
# npm（推荐）
npm install -g fnva

# Cargo
cargo install fnva
```

或前往 [Releases](https://github.com/Protagonistss/fnva/releases) 下载对应平台的二进制，加入 `PATH`。

## Shell 集成

安装 shell 集成后，打开新终端会自动恢复上次使用的 CC/Java 环境变量，且 `fnva <type> use <name>` 无需 `eval` 包裹即可生效。详细请参考 [Shell 集成指南](docs/user-guide/shell-integration.md)。

## 使用快速入门

### Java
- 扫描: `fnva java scan`
- 列表: `fnva java list`
- 切换: `eval "$(fnva java use jdk-17)"`
- 设置默认: `fnva java default jdk-17`

### Claude Code (CC) & LLM
- 列表: `fnva cc list`
- 切换: `eval "$(fnva cc use glmcc)"`
- 添加 LLM: `fnva llm add --name openai-dev --provider openai --api-key "sk-..." --model gpt-4`

## 配置

配置文件位于 `~/.fnva/config.toml` (Windows: `%USERPROFILE%\.fnva\config.toml`)。

## 许可证

MIT License.

<div align="center">

# fnva - Fast Environment Version Alter

**极速、跨平台的命令行环境版本切换工具**

[English](README_en.md) · [架构文档](docs/architecture/core-design.md) · [Releases](https://github.com/Protagonistss/fnva/releases)

[![npm version](https://img.shields.io/npm/v/fnva)](https://www.npmjs.com/package/fnva)
[![crates.io](https://img.shields.io/crates/v/fnva)](https://crates.io/crates/fnva)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

<!-- TODO: Add demo.gif here -->

</div>

fnva 是一个面向 Java、Maven 和 Claude Code (CC) 场景的跨平台环境切换工具。基于 Rust 编写，具备**极速启动**、**零依赖**的特点，通过生成 shell 片段完成环境激活，完全**无常驻后台进程**。

## 核心特性

- ⚡ **极速与零依赖**: 单一静态二进制文件。
- 🔄 **会话与全局切换**: 支持当前终端会话级别的切换，也可设置全局默认环境。
- 🐚 **全平台 Shell 支持**: 原生支持 PowerShell、Bash、Zsh、Fish、CMD。
- 🧠 **自动恢复**: 打开新终端自动恢复上次使用的环境。
- ☕ **智能 Java 管理**: 扫描本地 JDK，并支持通过 Adoptium 清华镜像源自动下载、安装和管理不同版本的 JDK。
- 📦 **Maven 版本管理**: 支持基于清华镜像源动态发现多版本、下载安装和一键切换。
- 🔌 **Tool Source Protocol (TSP)**: 提供工具无关协议，支持 Java 和 Maven 模块的动态发现与通用下载安装。

## 文档导航

- [架构设计与原理](docs/architecture/core-design.md)
- [开发路线图](docs/development/roadmap.md)
- [贡献指南](docs/development/contributing.md)
- [工具源协议规范](docs/spec/tool-source-protocol.md)
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

安装 shell 集成后，打开新终端会自动恢复上次使用的 CC/Java/Maven 环境变量，且 `fnva <type> use <name>` 无需 `eval` 包裹即可生效。详细请参考 [Shell 集成指南](docs/user-guide/shell-integration.md)。

```bash
# 添加到 shell 配置文件（bash/zsh）:
eval "$(fnva env)"
```

## 使用快速入门

### Java
- 扫描本地 JDK: `fnva java scan`
- 远程版本列表: `fnva java ls-remote`
- 自动安装: `fnva java install 17`
- 本地列表: `fnva java list`
- 切换版本: `fnva java use 17` （未安装 shell 集成时使用 `eval "$(fnva java use 17)"`）
- 设置默认: `fnva java default 17`

### Maven
- 远程版本列表: `fnva maven ls-remote`
- 自动安装: `fnva maven install 3.9.16`
- 本地列表: `fnva maven list`
- 切换版本: `fnva maven use 3.9.16` （未安装 shell 集成时使用 `eval "$(fnva maven use 3.9.16)"`）
- 刷新版本缓存: `fnva maven refresh`

### Claude Code (CC)
- 本地列表: `fnva cc list`
- 切换环境: `fnva cc use mycc` （未安装 shell 集成时使用 `eval "$(fnva cc use mycc)"`）

## 配置

用户配置位于 `~/.fnva/config.toml` (Windows: `%USERPROFILE%\.fnva\config.toml`)。

## 许可证

MIT License.

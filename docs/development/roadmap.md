# 开发方向与路线图 (Roadmap)

本文档记录 fnva 未来的开发演进方向和计划中的特性。

## 当前重点
- 完善现有的多语言 Shell 集成机制的稳定性。
- 稳定 Claude Code (CC) 切换模板并根据需要扩展。
- 优化并扩展基于 TSP (Tool Source Protocol) 的 Maven 与 Java 动态版本管理与快速安装。
- 优化跨平台构建流（Windows / macOS / Linux）。

## 规划中的特性 (Planned Features)
- [ ] **多环境组合配置 (Profiles)**: 允许用户定义一组环境（如 `Java 17` + `Maven 3.9` + `CC mycc`），并一键同时切换。
- [ ] **项目级隔离 (Local Config)**: 支持读取当前目录下的 `.fnva.toml` 实现进入目录自动切换（类似 `nvm use` 自动读取 `.nvmrc`）。
- [ ] **插件化架构 (Plugins)**: 允许社区通过简单脚本扩展 fnva 支持的环境类型（如 Node.js / Python 等）。

## 技术债与优化点
- 进一步优化二进制体积和启动速度。
- 提升测试覆盖率，补充针对特定 Shell 行为的 E2E 测试。

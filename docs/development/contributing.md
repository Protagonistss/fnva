# 贡献指南 (Contributing)

感谢您对 fnva 项目的关注！我们欢迎任何形式的贡献：提交 Bug、提供建议、改进文档或直接提交 PR。

## 本地构建与测试

fnva 使用标准 Rust 工具链进行开发。

1. **安装 Rust**: 请确保已安装最新的稳定版 Rust (推荐通过 rustup)。
2. **克隆项目**:
   ```bash
   git clone https://github.com/Protagonistss/fnva.git
   cd fnva
   ```
3. **格式化与代码检查**:
   ```bash
   cargo fmt
   cargo clippy --all-targets -- -D warnings
   ```
4. **运行单元测试**:
   ```bash
   cargo test
   ```
5. **本地编译**:
   ```bash
   cargo build --release
   ```
   编译产物位于 `target/release/fnva`。

## 代码规范

- 遵循 Rust 标准风格，所有代码在提交前必须通过 `cargo fmt` 和 `cargo clippy`。
- 新增特性需包含相应的单元测试。
- 核心修改需要同步更新相关文档（如 `README.md` 或 `docs/` 下的文件）。

## 提交与 Pull Request

- Commit 信息使用 [Conventional Commits](https://www.conventionalcommits.org/)，与现有提交历史保持一致，例如：`feat(cc): ...`、`fix(scripts): ...`、`refactor(core): ...`、`docs: ...`、`chore: bump version`。小修建议 squash 后再推送。
- PR 描述请使用模板（`.github/PULL_REQUEST_TEMPLATE.md`），包含：背景 / 动机、主要变更、测试结果、风险与回滚方案；关联 Issue / Discussion 请贴链接。
- 涉及安装 / 卸载脚本、二进制分发、shell 集成模板等影响面大的改动，"风险与回滚"必填。

## Discussions

Discussion 用于非 Issue 的交流（想法、问答、公告等）。请按分类发帖，与 Issue 区分：

- 📢 **Announcements**：版本发布、重要变更通知。
- 💡 **Ideas**：新功能 / 改进建议——方向未定时先在这里讨论，达成共识后再开 Issue / PR。
- 🙏 **Q&A**：使用问题、求助（非明确的 bug 缺陷）。
- 🙌 **Show and tell**：用法分享、集成案例。
- 💬 **General**：不属于以上类别的其他话题。
- 📊 **Polls**：征求意见 / 投票。

> 明确的 bug 或功能请求请提 Issue；需要讨论方向、收集意见的话题发 Discussion。

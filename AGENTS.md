# Repository Guidelines

## 项目结构与模块组织
- `src/`：Rust 源码。`cli/` 负责命令行解析与入口，`core/` 管理环境切换与会话，`environments/` 提供 Java/LLM 等环境管理，`infrastructure/` 处理配置与脚本生成，`utils/` 存放通用工具。
- `scripts/`：Node/脚本工具（安装、卸载、平台构建），`bin/` 为 npm 用户提供的入口，`platforms/` 存放按平台打包的二进制。
- `config/` 示例配置，`docs/` 文档素材，`target/` 为构建输出目录（不入库）。
- 工作目录建议始终在仓库根目录执行命令，避免相对路径失效。

## 构建、测试与开发命令
- `cargo fmt && cargo clippy --all-targets -- -D warnings`：格式化并做严格静态检查。
- `cargo build --release`：本机发布构建，产物在 `target/release/fnva[.exe]`。
- `npm run build:platforms`：跨平台构建脚本（Node 实现，可在 PowerShell/Git Bash/Unix 下运行），生成 `platforms/<os>-<arch>/fnva[.exe]`。
- `npm run build`：本地打包流程（依赖脚本目录），发布前可本地验证。
- `cargo test`：执行测试套件，建议与关键功能改动绑定。

## 代码风格与命名约定
- Rust 统一 `rustfmt`/`clippy` 规则（见 `.rustfmt.toml`、`.clippy.toml`）；优先显式错误处理，不吞异常。
- 文件与模块：`snake_case`；类型/trait：`PascalCase`；常量：`SCREAMING_SNAKE_CASE`；测试函数以 `test_<行为>` 命名。
- Node/脚本保持无 BOM、UTF-8，日志使用简短英文，避免 shell 循环递归。
- 仅在复杂分支或平台差异处添加简短注释，保持可读性。

## 测试指引
- 框架：标准 Rust 单元/集成测试（`cargo test`）。针对解析、环境切换、脚本生成、权限检查等关键路径添加用例。
- 倾向纯函数/字符串比对测试，避免在 CI 中依赖真实 shell 状态；必要时使用临时目录隔离文件系统副作用。
- 新增平台构建逻辑时，至少验证编译成功并检查生成二进制存在；无法运行的交叉编译目标应在日志中说明。

## Commit 与 Pull Request
- Commit 信息使用祈使句、单一主题，如：`Fix zsh integration recursion`、`Add linux arm64 build deps`。小修建议 squash 后再推送。
- PR 描述需包含：背景/动机、主要变更点、测试结果（列出运行的命令）、风险与回滚方案（尤其是安装/卸载脚本、二进制分发）。
- 如有关联 Issue/工单请在描述中链接；涉及终端交互或错误信息时附上关键日志或截图。

## 安全与配置提示
- 用户配置位于 `~/.fnva/config.toml`（Windows：`%USERPROFILE%\\.fnva\\config.toml`）。不要提交真实密钥，使用环境变量占位（如 `${OPENAI_API_KEY}`）。
- Shell 集成文件带有标记 `# fnva auto integration (added by npm install)`，卸载脚本会依据该标记清理；若手动编辑，请保留标记便于卸载。
- 发布前确认本地 `npm run build:platforms` 与 CI 输出一致；CI 打包产物应直接落在 `platforms/<os>-<arch>/`，确保 npm 发包和 GitHub Release 结构一致。

## 对话
- 优先使用中文

# 核心架构设计 (Core Design)

fnva 的设计目标是提供一个轻量、极速的跨平台环境版本切换工具。

## 1. 核心架构与工作原理

fnva 采用 Rust 编写，不依赖任何常驻的后台进程（Daemon）。它的核心工作机制是通过命令行执行，输出特定平台的 shell 脚本代码，然后由宿主 shell (`eval` 或 `source`) 执行这些代码，从而改变当前终端会话的环境变量。

工作流图解：
1. 用户输入命令：`fnva java use jdk-17`
2. fnva 解析命令，读取配置 `~/.fnva/config.toml`
3. fnva 组装对应 shell（如 Bash 或 PowerShell）的变量导出命令（例如 `export JAVA_HOME=...`）
4. fnva 将组装好的字符串输出到 stdout
5. 宿主 shell 通过 `eval "$(fnva ...)"` 捕获输出并在当前上下文中执行，完成环境变量更新。

## 2. 模块职责划分

代码主要集中在 `src/` 目录下，按功能领域划分：

- **CLI 层 (`src/cli/`)**: 负责命令行参数解析 (使用 `clap`) 和输出格式化。
- **Core 层 (`src/core/`)**: 会话管理与环境切换的核心逻辑入口。
- **Environments 层 (`src/environments/`)**: 具体技术栈的实现。
  - `java/`: JDK 的扫描、验证和切换。
  - `cc/`: Claude Code 的端点与 Key 配置。
  - `llm/`: 通用大模型的环境配置。
- **Infrastructure 层 (`src/infrastructure/`)**:
  - `config.rs`: 统一配置文件读写。
  - `shell/`: Shell 脚本生成工厂，负责生成 Bash/Zsh/Fish/PowerShell/CMD 的特定适配脚本。

## 3. 终端自动恢复机制

通过 `fnva env shell-integration` 生成的集成脚本，会在 shell 启动时自动执行。该脚本会读取 `~/.fnva/current_envs.toml` 中记录的上次活跃环境，并自动应用这些环境变量，实现新开终端的无缝环境继承。

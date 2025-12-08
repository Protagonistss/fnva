# fnva - 快速环境版本切换工具

[English](README_en.md) · [文档](docs/) · [Releases](https://github.com/Protagonistss/fnva/releases)

fnva 是一个跨平台的环境切换工具，面向 Java、Claude Code (CC) 和通用 LLM 场景。Rust 编写，启动快、零依赖，通过生成 shell 片段完成环境激活，无常驻进程。

## 特性

- 管理多套 Java/CC/LLM 配置，支持会话激活与全局默认。
- 自动生成 PowerShell/Bash/Zsh/Fish/CMD 初始化脚本。
- 扫描本地 JDK、去重并按名称一键切换。
- 统一配置 CC 端点和 LLM API 密钥，导出环境变量。
- 单一静态二进制，无额外运行时依赖。

## 安装

```bash
# npm（推荐）
npm install -g fnva

# Cargo
cargo install fnva
```

或前往 [Releases](https://github.com/Protagonistss/fnva/releases) 下载对应平台的二进制，加入 `PATH`。

## Shell 集成

在 shell 启动时自动加载：

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

## 使用

### Java
- 扫描: `fnva java scan`
- 列表: `fnva java list`
- 切换(会话): `eval "$(fnva java use jdk-17)"`
- 设置默认: `fnva java default jdk-17`
- 手动添加: `fnva java add --name jdk-8 --home "C:\\Java\\jdk1.8.0" --description "Legacy JDK"`

### Claude Code (CC)
- 列表: `fnva cc list`
- 添加 (GLM-4 示例):
  ```bash
  fnva cc add glmcc '{
    "provider": "anthropic",
    "api_key": "your-api-key",
    "base_url": "https://open.bigmodel.cn/api/anthropic",
    "model": "glm-4.6",
    "description": "GLM-4"
  }'
  ```
- 切换: `eval "$(fnva cc use glmcc)"`

### LLM
- 添加: `fnva llm add --name openai-dev --provider openai --api-key "sk-..." --model gpt-4`
- 切换: `eval "$(fnva llm use openai-dev)"`

## 配置

- 路径: `~/.fnva/config.toml` (Windows: `%USERPROFILE%\.fnva\config.toml`)
- 示例:
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

## 命令速查

| 命令 | 作用 |
| --- | --- |
| `fnva env env` | 生成 shell 初始化片段 |
| `fnva <type> list` | 列出环境 (type: java/cc/llm) |
| `fnva <type> use <name>` | 输出激活环境的脚本 |
| `fnva <type> current` | 查看当前环境 |
| `fnva <type> default <name>` | 查看/设置默认 (java/cc) |
| `fnva <type> remove <name>` | 删除环境 |
| `fnva java scan` | 扫描本机 JDK |
| `fnva config sync` | 同步/升级配置结构 |

## 构建与发布

- 格式/静态检查: `cargo fmt && cargo clippy --all-targets -- -D warnings`
- 测试: `cargo test`
- 构建: `cargo build --release`
- 跨平台打包: `npm run build:platforms`
- CI: 推送 `v*` tag 触发构建，发布到 GitHub Releases、npm (`NPM_TOKEN`) 与 crates.io (`CARGO_TOKEN`)。

## 许可证

MIT License.

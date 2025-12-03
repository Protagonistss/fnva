# fnva - Fast Environment Version Alter

fnva 是一个现代化的跨平台环境管理工具，支持 Java、Claude Code (CC) 和 LLM 环境的快速切换与管理。它采用 Rust 编写，具有高性能、零依赖和即时响应的特点。

## ✨ 核心特性

- **⚡ 高性能**：基于 Rust 构建，启动速度极快。
- **☕ Java 环境管理**：智能扫描、版本识别、快速切换 JDK。
- **🤖 Claude Code (CC) 支持**：专为 Claude Code 设计的多环境切换（支持 GLM-4, AnyCC, Kimi 等）。
- **🧠 LLM 环境配置**：统一管理 OpenAI, Anthropic, Local LLM 等 API 配置。
- **🐚 多 Shell 支持**：完美支持 PowerShell, Bash, Zsh, Fish, CMD。
- **🔄 自动加载**：Shell 启动时自动应用默认环境。
- **🔍 智能扫描**：自定义路径扫描，自动去重。

---

## 🚀 快速开始

### 1. 安装

#### 方式一：使用 NPM (推荐)

```bash
npm install -g fnva
```

#### 方式二：下载二进制文件

前往 [Releases 页面](https://github.com/Protagonistss/fnva/releases) 下载对应系统的二进制文件，解压并添加到系统的 PATH 环境变量中。

#### 方式三：源码编译

```bash
git clone https://github.com/Protagonistss/fnva.git
cd fnva
cargo build --release
# 产物位于 target/release/fnva (或 fnva.exe)
```

### 2. 初始化 (Shell 集成)

为了启用环境自动加载和切换功能，请根据您的 Shell 配置集成脚本。

**PowerShell:**
```powershell
# 添加到 $PROFILE
fnva env env --shell powershell | Out-String | Invoke-Expression
```

**Bash / Zsh:**
```bash
# 添加到 ~/.bashrc 或 ~/.zshrc
eval "$(fnva env env --shell bash)"
# 或 zsh
eval "$(fnva env env --shell zsh)"
```

**Fish:**
```fish
# 添加到 ~/.config/fish/config.fish
fnva env env --shell fish | source
```

---

## 📖 使用指南

### ☕ Java 环境管理

fnva 可以扫描并管理系统中的 JDK 版本。

- **扫描环境**
  ```bash
  fnva java scan
  ```
- **列出环境**
  ```bash
  fnva java list
  ```
- **切换环境 (当前会话)**
  ```bash
  # PowerShell
  fnva java use jdk-17 | Invoke-Expression

  # Bash/Zsh
  eval "$(fnva java use jdk-17)"
  ```
- **设置默认环境 (全局生效)**
  ```bash
  fnva java default jdk-17
  ```
- **手动添加环境**
  ```bash
  fnva java add --name jdk-8 --home "C:\Java\jdk1.8.0" --description "Legacy JDK"
  ```

### 🤖 Claude Code (CC) 环境

专为 Claude Code 工具链设计的环境切换功能，支持配置不同的 API 端点和密钥。

- **查看环境列表**
  ```bash
  fnva cc list
  ```

- **添加 CC 环境**
  
  *示例：添加 GLM-4 兼容环境*
  ```bash
  fnva cc add glmcc '{
    "provider": "anthropic",
    "api_key": "your-api-key",
    "base_url": "https://open.bigmodel.cn/api/anthropic",
    "model": "glm-4.6",
    "description": "智谱 GLM-4"
  }'
  ```

- **切换 CC 环境**
  ```bash
  # PowerShell
  fnva cc use glmcc | Invoke-Expression

  # Bash/Zsh
  eval "$(fnva cc use glmcc)"
  ```
  *此操作会设置 `ANTHROPIC_API_KEY`, `ANTHROPIC_BASE_URL` 等必要环境变量。*

- **常用预设**
  - `glmcc`: 智谱 GLM-4
  - `anycc`: AnyCC 代理
  - `kimicc`: Moonshot Kimi

### 🧠 LLM 环境管理

统一管理各类 LLM API 密钥和配置。

- **添加环境**
  ```bash
  fnva llm add --name openai-dev --provider openai --api-key "sk-..." --model gpt-4
  ```
- **切换环境**
  ```bash
  # PowerShell
  fnva llm use openai-dev | Invoke-Expression
  ```

---

## ⚙️ 配置说明

首次运行后，配置文件会自动创建。

- **路径**: 
  - Windows: `~/.fnva/config.toml`
  - macOS/Linux: `~/.fnva/config.toml`

### 配置文件示例

```toml
# ~/.fnva/config.toml

# 自定义 Java 扫描路径
custom_java_scan_paths = [
    "D:\\Environment\\Java",
    "/opt/java"
]

# Java 环境定义
[[java_environments]]
name = "jdk-21"
java_home = "C:\\Program Files\\Java\\jdk-21"
description = "Oracle JDK 21"

# CC 环境定义
[[cc_environments]]
name = "glmcc"
provider = "anthropic"
api_key = "sk-..."
base_url = "https://open.bigmodel.cn/api/anthropic"
model = "glm-4.6"
```

---

## 🛠️ 常用命令速查

| 命令 | 说明 |
|------|------|
| `fnva env env` | 生成 Shell 初始化脚本 (用于配置 Shell 环境加载) |
| `fnva <type> list` | 列出指定类型的所有环境 (type: java/cc/llm) |
| `fnva <type> use <name>` | 生成切换环境的脚本 (需执行输出内容) |
| `fnva <type> current` | 查看当前激活的环境 |
| `fnva <type> default <name>` | 查看或设置默认环境 (仅 java/cc) |
| `fnva <type> remove <name>` | 删除环境配置 |
| `fnva java scan` | 扫描本机 Java 环境 |
| `fnva config sync` | 同步/更新配置文件结构 |

## 📄 许可证

MIT License

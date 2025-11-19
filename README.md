# fnva - Fast Environment Version Alter

跨平台环境管理工具，支持 Java、LLM 和 Claude Code 环境，具有默认环境设置和自动加载功能。

## 功能特性

- ✅ **Java 环境管理**：快速切换不同版本的 JDK
- ✅ **LLM 环境管理**：支持多 LLM 提供商配置切换
- ✅ **Claude Code (CC) 环境管理**：专门为 Claude Code 设计的环境切换
- ✅ **默认环境支持**：支持设置默认环境
- ✅ **自动加载**：新 Shell 会话自动加载默认环境
- ✅ **智能扫描**：高效扫描系统 Java 安装，支持自定义路径
- ✅ **跨平台支持**：Windows、macOS、Linux
- ✅ **多 Shell 支持**：bash、zsh、fish、PowerShell、CMD
- ✅ **配置化扫描**：支持配置文件和环境变量自定义扫描路径
- ✅ **高效去重**：智能去除重复的环境条目

## 安装

### 方式一：通过 npm 安装（推荐）

```bash
# 全局安装
npm install -g fnva

# 使用 yarn
yarn global add fnva

# 使用 pnpm
pnpm add -g fnva

function fnva {
  if ($args.Count -ge 2 -and ($args[0] -eq "java" -or $args[0] -eq "llm" -or $args[0] -eq "cc") -and ($args[1] -eq "use")) {
      $tempFile = "$env:TEMP\fnva_script_$(Get-Random).ps1"

      $env:FNVAAUTOMODE = "1"
      try {
          cmd.exe /c "set FNVA_AUTO_MODE=%FNVAAUTOMODE% && fnva $args" | Out-File -FilePath $tempFile -Encoding UTF8
          & $tempFile
      } finally {
          $env:FNVAAUTOMODE = ""
          Remove-Item $tempFile -ErrorAction SilentlyContinue
      }
  } else {
      $env:FNVAAUTOMODE = "1"
      try {
          cmd.exe /c "set FNVA_AUTO_MODE=%FNVAAUTOMODE% && fnva $args"
      } finally {
          $env:FNVAAUTOMODE = ""
      }
  }
}
```

### 方式二：从 Releases 下载二进制文件

1. 访问 [GitHub Releases](https://github.com/your-repo/fnva/releases)
2. 下载对应平台的二进制文件：
   - Windows: `fnva-win32-x64.exe`
   - macOS: `fnva-darwin-x64` 或 `fnva-darwin-arm64`
   - Linux: `fnva-linux-x64`

3. 将二进制文件重命名为 `fnva`（Windows 下为 `fnva.exe`）

4. 添加到 PATH 环境变量（详见下面的配置步骤）

### 方式三：从源码构建（开发者）

**前置要求：**
- **Rust** 1.70+
- **系统依赖**：
  - Linux: `pkg-config`, `libssl-dev`, `build-essential`
  - macOS: Xcode Command Line Tools
  - Windows: Microsoft Visual Studio C++ Build Tools

```bash
# 克隆仓库
git clone git@github.com:Protagonistss/fnva.git
cd fnva

# 构建
cargo build --release

# 二进制文件位置：
# Windows: target\release\fnva.exe
# macOS/Linux: target/release/fnva
```

### 安装后配置

#### 1. 验证安装

```bash
fnva --version
```

#### 2. Shell 集成

为了获得最佳体验，需要配置 Shell 集成。这会让 fnva 在新的 Shell 会话中自动加载环境。

**PowerShell（推荐）：**
```powershell
# 添加到 PowerShell Profile
fnva env env --shell powershell | Out-String | Invoke-Expression

# 或手动添加到 $PROFILE
echo 'fnva env env --shell powershell | Out-String | Invoke-Expression' >> $PROFILE
```

**Bash/Zsh：**
```bash
# 添加到 ~/.bashrc 或 ~/.zshrc
echo 'eval "$(fnva env env --shell bash)"' >> ~/.bashrc
# 或
echo 'eval "$(fnva env env --shell zsh)"' >> ~/.zshrc

# 重新加载配置
source ~/.bashrc  # 或 source ~/.zshrc
```

**Fish：**
```fish
# 添加到 ~/.config/fish/config.fish
echo 'fnva env env --shell fish | source' >> ~/.config/fish/config.fish
```

#### 3. 配置文件

首次运行时，fnva 会自动创建配置文件：

```bash
# 配置文件位置
Linux/macOS: ~/.fnva/config.toml
Windows:     %USERPROFILE%\.fnva\config.toml
```

#### 4. 测试安装

```bash
# 列出所有环境类型
fnva env list-types

# 查看 Java 环境
fnva java list

# 查看 CC 环境
fnva cc list

# 查看 LLM 环境
fnva llm list
```

## 使用方法

### Java 环境管理

#### 列出所有 Java 环境

```bash
fnva java list
```

#### 添加 Java 环境

```bash
fnva java add --name jdk-17 --home /usr/lib/jvm/java-17-openjdk --description "OpenJDK 17"
```

#### 切换到 Java 环境

```bash
# Bash / Zsh
eval "$(fnva java use jdk-17)"

# Fish
fnva java use jdk-17 --shell fish | source

# PowerShell
fnva java use jdk-17 --shell powershell | Invoke-Expression

# CMD
fnva java use jdk-17 --shell cmd > %TEMP%\fnva_use.cmd && call %TEMP%\fnva_use.cmd
```

#### 设置默认 Java 环境

```bash
# 设置默认环境
fnva java default jdk-21

# 查看当前默认环境
fnva java default

# 清除默认设置
fnva java default --unset
```

#### 查看当前激活的环境

```bash
fnva java current
```

#### 删除 Java 环境

```bash
fnva java remove jdk-17
```

#### 扫描系统中的 Java 安装

```bash
fnva java scan
```

**扫描功能详解：**

**基础扫描：**
- 自动检测系统标准 Java 安装路径
- 扫描用户主目录下的 `.fnva/java-packages`
- 检查 PATH 环境变量中的 Java 可执行文件

**自定义扫描路径：**

1. **配置文件方式**（推荐）：
   ```toml
   # ~/.fnva/config.toml
   custom_java_scan_paths = [
       "D:\\tools\\java",
       "/opt/custom/java",
       "/home/user/my-jdks"
   ]
   ```

2. **环境变量方式**：
   ```bash
   # 临时添加扫描路径
   export FNVA_SCAN_PATHS="/path/to/jdk1:/path/to/jdk2"
   fnva java scan

   # Windows
   set FNVA_SCAN_PATHS=D:\tools\java;E:\other\java
   fnva java scan
   ```

**扫描性能：**
- 🔒 **安全**：只扫描指定路径，不进行全盘搜索
- ⚡ **快速**：使用高效的去重算法，避免重复处理
- 🎯 **精确**：智能识别 Java 安装，过滤无效路径

**支持的扫描路径：**
- Windows：`C:\Program Files\Java`、`C:\Program Files\Eclipse Adoptium` 等
- macOS：`/Library/Java/JavaVirtualMachines`、`/opt/homebrew/Caskroom` 等
- Linux：`/usr/lib/jvm`、`/opt/java`、`/usr/local/java` 等

### LLM 环境管理

#### 列出所有 LLM 环境

```bash
fnva llm list
```

#### 查看支持的提供商

```bash
fnva llm providers
```

支持的提供商：
- `openai` - OpenAI API
- `anthropic` - Anthropic Claude API
- `azure-openai` - Azure OpenAI
- `google-gemini` - Google Gemini
- `cohere` - Cohere API
- `mistral` - Mistral AI
- `ollama` - Ollama (本地部署)

#### 添加 LLM 环境

```bash
# OpenAI
fnva llm add \
  --name openai-dev \
  --provider openai \
  --api-key "${OPENAI_API_KEY}" \
  --model gpt-4 \
  --temperature 0.7

# Anthropic
fnva llm add \
  --name anthropic-prod \
  --provider anthropic \
  --api-key "${ANTHROPIC_API_KEY}" \
  --model claude-3-opus-20240229

# Ollama (本地)
fnva llm add \
  --name ollama-local \
  --provider ollama \
  --base-url http://localhost:11434 \
  --model llama2
```

#### 切换到 LLM 环境

```bash
# 自动检测 shell
eval "$(fnva llm use openai-dev)"

# 指定 shell (PowerShell)
fnva llm use openai-dev --shell powershell | Invoke-Expression
```

#### 删除 LLM 环境

```bash
fnva llm remove openai-dev
```

### Claude Code (CC) 环境管理

专门为 Claude Code 设计的环境管理功能，支持多种 Claude Code 兼容服务的环境切换。

#### 列出所有 CC 环境

```bash
fnva cc list
```

#### 添加 CC 环境

```bash
# 方法一：使用 JSON 配置
fnva cc add glmcc '{
  "provider": "anthropic",
  "api_key": "your-api-key",
  "base_url": "https://open.bigmodel.cn/api/anthropic",
  "model": "glm-4.6",
  "description": "GLM-4.6 Claude Code 环境"
}'

# 方法二：直接编辑配置文件
# 编辑 ~/.fnva/config.toml，添加：
# [[cc_environments]]
# name = "glmcc"
# provider = "anthropic"
# api_key = "your-api-key"
# base_url = "https://open.bigmodel.cn/api/anthropic"
# model = "glm-4.6"
# description = "GLM-4.6 Claude Code 环境"
```

#### 切换到 CC 环境

```bash
# PowerShell（推荐）
fnva cc use glmcc --shell powershell | Invoke-Expression

# Bash/Zsh
eval "$(fnva cc use glmcc)"

# Fish
fnva cc use glmcc --shell fish | source

# CMD
fnva cc use glmcc --shell cmd > %TEMP%\fnva_cc.cmd && call %TEMP%\fnva_cc.cmd
```

#### Manage default CC environment

```bash
# Set default CC environment
fnva cc default glmcc

# Show current default CC environment
fnva cc default

# Unset default CC environment
fnva cc default --unset
```

#### 查看当前激活的 CC 环境

```bash
fnva cc current
```

#### 删除 CC 环境

```bash
fnva cc remove glmcc
```

#### 预配置的 CC 环境

fnva 提供了一些常用的 CC 环境配置：

- **glmcc**: GLM-4.6 智谱 AI Claude Code 兼容服务
- **anycc**: AnyCC 通用 Claude Code 代理服务
- **kimicc**: Kimi AI Claude Code 兼容服务

#### 环境变量说明

CC 环境切换会设置以下环境变量：

- `ANTHROPIC_AUTH_TOKEN`: Claude Code 认证令牌
- `ANTHROPIC_BASE_URL`: Claude Code API 基础 URL
- `ANTHROPIC_DEFAULT_SONNET_MODEL`: 默认使用的模型
- `API_TIMEOUT_MS`: API 请求超时时间
- `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`: 禁用非必要网络流量

#### 配置示例

```toml
[[cc_environments]]
name = "glmcc"
provider = "anthropic"
api_key = "your-glm-api-key"
base_url = "https://open.bigmodel.cn/api/anthropic"
model = "glm-4.6"
description = "GLM-4.6 Claude Code 环境"

[[cc_environments]]
name = "anycc"
provider = "anthropic"
api_key = "your-anycc-api-key"
base_url = "https://your-anycc-proxy.com"
model = "claude-sonnet-4-5"
description = "AnyCC 代理服务"

[[cc_environments]]
name = "kimicc"
provider = "anthropic"
api_key = "your-kimi-api-key"
base_url = "https://api.moonshot.cn/anthropic"
model = "kimi-k2-turbo-preview"
description = "Kimi AI Claude Code 环境"
```

## 配置文件

配置文件位置：
- **Linux/macOS**: `~/.fnva/config.toml`
- **Windows**: `%USERPROFILE%\.fnva\config.toml`

首次运行时会自动创建配置文件。

### 配置示例

查看 `config/config.toml.example` 获取完整配置示例。

```toml
# Java 环境配置
[[java_environments]]
name = "jdk-17"
java_home = "/usr/lib/jvm/java-17-openjdk"
description = "OpenJDK 17"

# LLM 环境配置
[[llm_environments]]
name = "openai-dev"
provider = "openai"
api_key = "${OPENAI_API_KEY}"
base_url = "https://api.openai.com/v1"
model = "gpt-4"
temperature = 0.7
max_tokens = 2000
description = "OpenAI 开发环境"

# Claude Code (CC) 环境配置
[[cc_environments]]
name = "glmcc"
provider = "anthropic"
api_key = "${GLM_API_KEY}"
base_url = "https://open.bigmodel.cn/api/anthropic"
model = "glm-4.6"
description = "GLM-4.6 Claude Code 环境"

[[cc_environments]]
name = "anycc"
provider = "anthropic"
api_key = "sk-your-api-key"
base_url = "https://your-proxy.com"
model = "claude-sonnet-4-5"
description = "AnyCC 代理服务"

# 仓库配置
[repositories]
java = [
    "https://mirrors.aliyun.com/eclipse/temurin-compliance/temurin",
    "https://api.adoptium.net/v3"
]
maven = [
    "https://maven.aliyun.com/repository/public",
    "https://search.maven.org/solrsearch/select"
]
```

### 常用命令速查

| 命令 | 功能 | 示例 |
|------|------|------|
| `fnva java list` | 列出 Java 环境 | `fnva java list` |
| `fnva java use <name>` | 切换 Java 环境 | `fnva java use jdk21` |
| `fnva java default <name>` | 设置默认 Java | `fnva java default jdk21` |
| `fnva cc list` | 列出 CC 环境 | `fnva cc list` |
| `fnva cc use <name>` | 切换 CC 环境 | `fnva cc use glmcc` |
| `fnva cc default <name>` | 设置默认 CC 环境 | `fnva cc default glmcc` |
| `fnva llm list` | 列出 LLM 环境 | `fnva llm list` |
| `fnva llm use <name>` | 切换 LLM 环境 | `fnva llm use openai-dev` |
| `fnva env switch <type> <name>` | 通用切换 | `fnva env switch java jdk17` |

## 许可证

MIT License


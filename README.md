# fnva - Fast Node Version Manager for Java

类似 fnm 的跨平台 Java 环境管理工具，支持默认环境设置和自动加载。

## 功能特性

- ✅ **Java 环境管理**：快速切换不同版本的 JDK
- ✅ **默认环境支持**：类似 fnm，支持设置默认 Java 环境
- ✅ **自动加载**：新 Shell 会话自动加载默认环境
- ✅ **LLM 环境管理**：支持多 LLM 提供商配置切换
- ✅ **跨平台支持**：Windows、macOS、Linux
- ✅ **多 Shell 支持**：bash、zsh、fish、PowerShell、CMD
- ✅ **自动扫描**：自动检测系统中的 Java 安装
- ✅ **环境变量引用**：支持 `${VAR_NAME}` 格式引用系统环境变量

## 安装

### 从源码构建（推荐）

```bash
git clone <repository-url>
cd fnva
cargo build --release
```

### 通过 Cargo 安装

```bash
cargo install --path .
```

### 添加到 PATH

将二进制文件复制到系统 PATH 中：

```bash
# Linux/macOS
sudo cp target/release/fnva /usr/local/bin/fnva

# 或添加到 ~/.bashrc 或 ~/.zshrc
export PATH="$PATH:$(pwd)/target/release"
```

Windows 用户需要将 `target\release\fnva.exe` 添加到 PATH 环境变量中。

## 使用方法

### Java 环境管理

#### 列出所有 Java 环境

```bash
fnva java list
```

#### 扫描系统中的 Java 安装

```bash
fnva java scan
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

#### 设置默认 Java 环境（类似 fnm）

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

## 配置文件

配置文件位置：
- **Linux/macOS**: `~/.fnva/config.toml`
- **Windows**: `%USERPROFILE%\.fnva\config.toml`

首次运行时会自动创建配置文件。

### 配置示例

查看 `config/config.toml.example` 获取完整配置示例。

```toml
[[java_environments]]
name = "jdk-17"
java_home = "/usr/lib/jvm/java-17-openjdk"
description = "OpenJDK 17"

[[llm_environments]]
name = "openai-dev"
provider = "openai"
api_key = "${OPENAI_API_KEY}"
base_url = "https://api.openai.com/v1"
model = "gpt-4"
temperature = 0.7
```

## 环境变量引用

在配置文件中可以使用 `${VAR_NAME}` 格式引用系统环境变量：

```toml
[[llm_environments]]
name = "openai-prod"
provider = "openai"
api_key = "${OPENAI_API_KEY}"  # 从系统环境变量读取
```

## Shell 集成（fnm 风格）

### PowerShell（推荐）

在你的 PowerShell Profile 中添加以下内容以启用 fnm 风格的自动环境切换：

```powershell
# fnva 环境集成（类似 fnm env）
fnva env env --shell powershell | Out-String | Invoke-Expression
```

#### 功能特性

- **自动加载默认环境**：新 PowerShell 会话自动加载设置的默认 Java 环境
- **环境持久化**：重启 PowerShell 后自动恢复上次的 Java 环境
- **智能切换函数**：提供 `fnva java use` 交互式切换功能
- **Shell 函数集成**：自动添加 PowerShell 函数用于环境切换

#### 使用示例

```powershell
# 1. 设置默认环境
fnva java default jdk21

# 2. 重启 PowerShell 后会自动加载默认环境
# 显示: "Loading default Java environment: jdk21"

# 3. 交互式切换
fnva java use jdk17

# 4. 查看当前环境
fnva java current
```

### Bash/Zsh

在 `~/.bashrc` 或 `~/.zshrc` 中添加：

```bash
# fnva 环境集成
eval "$(fnva env env --shell bash)"

# 或使用别名快速切换
alias java17='eval "$(fnva java use jdk-17)"'
alias java11='eval "$(fnva java use jdk-11)"'
```

### Fish

在 `~/.config/fish/config.fish` 中添加：

```fish
# fnva 环境集成
fnva env env --shell fish | source

# 或定义函数
function java17
    fnva java use jdk-17 | source
end

function java11
    fnva java use jdk-11 | source
end
```

## 工作原理

### 默认环境管理

fnva 类似 fnm 的工作方式：

1. **设置默认环境**：
   ```bash
   fnva java default jdk21
   ```

2. **Shell 集成**：在 Shell Profile 中添加环境切换脚本

3. **自动加载**：新 Shell 会话自动检测并加载默认环境

4. **环境持久化**：环境配置保存在 `~/.fnva/config.toml` 中

### 配置文件位置

- **Linux/macOS**: `~/.fnva/config.toml`
- **Windows**: `%USERPROFILE%\.fnva\config.toml`

默认环境配置示例：
```toml
default_java_env = "jdk21.0.6"

[[java_environments]]
name = "jdk21.0.6"
java_home = "E:\\env\\jdks\\jdk-21.0.6"
description = "Java 21.0.6 LTS"
```

## 许可证

MIT License


# 环境切换工具 (nva)

跨平台环境切换工具，支持 Java 和 LLM 环境配置管理。

## 功能特性

- ✅ **Java 环境管理**：快速切换不同版本的 JDK
- ✅ **LLM 环境管理**：支持多 LLM 提供商配置切换
- ✅ **跨平台支持**：Windows、macOS、Linux
- ✅ **多 Shell 支持**：bash、zsh、fish、PowerShell、CMD
- ✅ **自动扫描**：自动检测系统中的 Java 安装
- ✅ **环境变量引用**：支持 `${VAR_NAME}` 格式引用系统环境变量

## 安装

### 从源码构建

```bash
git clone <repository-url>
cd cool-utils
cargo build --release
```

编译后的二进制文件位于 `target/release/nva`。

### 添加到 PATH

将二进制文件复制到系统 PATH 中，或创建符号链接：

```bash
# Linux/macOS
sudo ln -s $(pwd)/target/release/nva /usr/local/bin/nva

# 或添加到 ~/.bashrc 或 ~/.zshrc
export PATH="$PATH:$(pwd)/target/release"
```

## 使用方法

### Java 环境管理

#### 列出所有 Java 环境

```bash
nva java list
```

#### 扫描系统中的 Java 安装

```bash
nva java scan
```

#### 添加 Java 环境

```bash
nva java add --name jdk-17 --home /usr/lib/jvm/java-17-openjdk --description "OpenJDK 17"
```

#### 切换到 Java 环境

```bash
# 自动检测 shell
eval "$(nva java use jdk-17)"

# 指定 shell
eval "$(nva java use jdk-17 --shell bash)"
```

#### 删除 Java 环境

```bash
nva java remove jdk-17
```

### LLM 环境管理

#### 列出所有 LLM 环境

```bash
nva llm list
```

#### 查看支持的提供商

```bash
nva llm providers
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
nva llm add \
  --name openai-dev \
  --provider openai \
  --api-key "${OPENAI_API_KEY}" \
  --model gpt-4 \
  --temperature 0.7

# Anthropic
nva llm add \
  --name anthropic-prod \
  --provider anthropic \
  --api-key "${ANTHROPIC_API_KEY}" \
  --model claude-3-opus-20240229

# Ollama (本地)
nva llm add \
  --name ollama-local \
  --provider ollama \
  --base-url http://localhost:11434 \
  --model llama2
```

#### 切换到 LLM 环境

```bash
# 自动检测 shell
eval "$(nva llm use openai-dev)"

# 指定 shell (PowerShell)
nva llm use openai-dev --shell powershell | Invoke-Expression
```

#### 删除 LLM 环境

```bash
nva llm remove openai-dev
```

## 配置文件

配置文件位置：
- **Linux/macOS**: `~/.nva/config.toml`
- **Windows**: `%USERPROFILE%\.nva\config.toml`

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

## Shell 集成

### Bash/Zsh

在 `~/.bashrc` 或 `~/.zshrc` 中添加：

```bash
# 快速切换 Java 环境
alias java17='eval "$(nva java use jdk-17)"'
alias java11='eval "$(nva java use jdk-11)"'

# 快速切换 LLM 环境
alias llm-openai='eval "$(nva llm use openai-dev)"'
alias llm-anthropic='eval "$(nva llm use anthropic-prod)"'
```

### Fish

在 `~/.config/fish/config.fish` 中添加：

```fish
function java17
    nva java use jdk-17 | source
end

function llm-openai
    nva llm use openai-dev | source
end
```

### PowerShell

在 `$PROFILE` 中添加：

```powershell
function Switch-Java {
    param([string]$Name)
    nva java use $Name | Invoke-Expression
}

function Switch-Llm {
    param([string]$Name)
    nva llm use $Name | Invoke-Expression
}
```

## 许可证

MIT License


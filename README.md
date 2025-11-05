# 环境切换工具 (fnva)

跨平台环境切换工具，支持 Java 和 LLM 环境配置管理。

## 功能特性

- ✅ **Java 环境管理**：快速切换不同版本的 JDK
- ✅ **LLM 环境管理**：支持多 LLM 提供商配置切换
- ✅ **跨平台支持**：Windows、macOS、Linux
- ✅ **多 Shell 支持**：bash、zsh、fish、PowerShell、CMD
- ✅ **自动扫描**：自动检测系统中的 Java 安装
- ✅ **环境变量引用**：支持 `${VAR_NAME}` 格式引用系统环境变量

## 安装

### 通过 npm 安装（推荐）

```bash
npm install -g fnva

yarn global add fnva

pnpm add -g fnva
```

安装完成后，可以直接使用 `fnva` 命令。

### 从源码构建

#### 本地构建（当前平台）

```bash
git clone <repository-url>
cd cool-utils
npm run build
```

编译后的二进制文件位于 `platforms/<platform>/fnva`。

#### 构建所有平台

项目使用 GitHub Actions 自动构建所有平台的二进制文件。当创建版本标签时，会自动构建并发布到 npm。

**手动触发构建：**
1. 在 GitHub 上创建新的 Release 标签（例如 `v0.1.0`）
2. GitHub Actions 会自动构建所有平台
3. 构建完成后自动发布到 npm

**本地构建所有平台（需要交叉编译工具）：**
```bash
npm run build:all
```

注意：需要安装 `cross` 工具：`cargo install cross`

### 添加到 PATH

将二进制文件复制到系统 PATH 中，或创建符号链接：

```bash
# Linux/macOS
sudo ln -s $(pwd)/target/release/fnva /usr/local/bin/fnva

# 或添加到 ~/.bashrc 或 ~/.zshrc
export PATH="$PATH:$(pwd)/target/release"
```

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
`fnva java use <name>` prints shell commands for the target shell. Pipe/eval this output or wrap it in your profile for an fnm-style experience.


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

## Shell 集成

### Bash/Zsh

在 `~/.bashrc` 或 `~/.zshrc` 中添加：

```bash
# 快速切换 Java 环境
alias java17='eval "$(fnva java use jdk-17)"'
alias java11='eval "$(fnva java use jdk-11)"'

# 快速切换 LLM 环境
alias llm-openai='eval "$(fnva llm use openai-dev)"'
alias llm-anthropic='eval "$(fnva llm use anthropic-prod)"'
```

### Fish

在 `~/.config/fish/config.fish` 中添加：

```fish
function java17
    fnva java use jdk-17 | source
end

function llm-openai
    fnva llm use openai-dev | source
end
```

### PowerShell

#### 自动集成（推荐）

在你的 PowerShell profile (`C:\Users\Administrator\Documents\PowerShell\Microsoft.PowerShell_profile.ps1`) 中添加以下一行：

```powershell
fnva env --use-on-cd | Out-String | Invoke-Expression
```

```
function fnva {
    param(
        [Parameter(ValueFromRemainingArguments=$true)]
        [string[]]$Args
    )

    if ($Args.Count -ge 3 -and $Args[0] -eq "java" -and $Args[1] -eq "use") {
        $envName = $Args[2]
        $output = fnva.exe java use $envName --shell powershell 2>$null
        if ($output -is [array]) {
            $script = $output -join "`r`n"
        } else {
            $script = $output
        }

        if ($LASTEXITCODE -eq 0 -and $script -match "JAVA_HOME") {
            try {
                Invoke-Expression $script
                Write-Host "Switched to Java: $envName" -ForegroundColor Green
            } catch {
                Write-Error "Failed to execute switch script: $($_.Exception.Message)"
            }
        } else {
            Write-Output $output
        }
    } else {
        fnva.exe $Args
    }
}
```

重启 PowerShell 后即可享受自动 Java 环境切换！

#### 手动集成

在 `$PROFILE` 中添加函数：

```powershell
function Switch-Java {
    param([string]$Name)
    fnva java use $Name | Invoke-Expression
}

function Switch-Llm {
    param([string]$Name)
    fnva llm use $Name | Invoke-Expression
}
```

### Windows 自动切换功能

`fnva` 现在支持类似 `fnm` 的自动环境切换功能！

#### 快速开始

1. **添加到 PowerShell Profile**：
   ```powershell
   fnva env --use-on-cd | Out-String | Invoke-Expression
   ```

2. **重启 PowerShell**

3. **开始使用**：
   ```powershell
   # 设置 Java 环境
   fnva java use jdk21

   # 环境会自动保持激活状态
   # 重启 PowerShell 后自动恢复

   # 切换版本
   fnva java use jdk17
   ```

#### 新增功能

- **JSON 输出支持**：
  ```powershell
  fnva java current --json
  ```

- **自动环境切换**：环境状态持久化，重启后自动恢复
- **智能 PATH 管理**：自动清理旧的 Java 路径
- **增强错误处理**：包含回滚和验证机制

#### 工作原理

PowerShell Hook 会在每次显示提示符时：
1. 检查当前环境状态
2. 使用 `fnva java current --json` 获取环境信息
3. 智能切换环境（如需要）
4. 清理 PATH 并设置环境变量

## 许可证

MIT License


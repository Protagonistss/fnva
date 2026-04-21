# Shell 集成机制规格

## 概述

Shell 集成是 fnva 的核心机制，通过在 shell 配置文件中注入函数脚本来实现：
1. **Autoload**: 新终端启动时自动恢复上次的环境
2. **Wrapper**: 拦截 `fnva java/llm/cc use` 命令，捕获脚本输出并在当前 shell 执行

## Integration Script 结构

每个 shell 的 integration script 由三部分组成，**顺序固定，不可调换**：

```
1. fnva_autoload_default() 函数定义    ← 准备恢复逻辑
2. fnva_autoload_default 调用           ← 立即执行恢复
3. fnva() wrapper 函数定义              ← 供后续交互式使用
```

### 为什么这个顺序很重要

Autoload 在 wrapper 定义之前运行。此时 `fnva` 尚未被 wrapper 覆盖，
`command fnva` 和直接 `fnva` 都走外部命令。如果顺序反过来：
- Autoload 中的 `fnva cc use glmcc` 会触发 wrapper
- Wrapper 会把输出重定向到 temp_file 然后 source
- `env_script=$(fnva ...)` 捕获到的是空字符串（因为 wrapper 已处理了输出）
- `eval ""` 什么都不做，环境变量不会被设置

## Autoload 函数

### 职责

在 shell 启动时读取 `~/.fnva/current_envs.toml`，逐个恢复环境变量。

### Bash/Zsh 实现

```bash
_fnva_autoload_done=false
fnva_autoload_default() {
    if [[ $_fnva_autoload_done == "true" ]]; then return; fi
    _fnva_autoload_done=true

    local envs_file="$HOME/.fnva/current_envs.toml"
    if [[ -f "$envs_file" ]] && command -v fnva >/dev/null 2>&1; then
        local _restored=""
        while IFS='=' read -r key value; do
            key=$(echo "$key" | tr -d '[:space:]')
            value=$(echo "$value" | tr -d '[:space:]' | tr -d '"')
            [[ -z "$value" ]] && continue
            _FNVA_QUIET=1 eval "$(command fnva "$key" use "$value" 2>/dev/null)" >/dev/null 2>&1
            unset _FNVA_QUIET
            _restored="$_restored $value"
        done < "$envs_file"
        if [[ -n "$_restored" ]]; then
            echo "[fnva] restored:$_restored"
        fi
    fi
}
```

### Fish 实现

```fish
set -g _fnva_autoload_done false
function fnva_autoload_default
    if test $_fnva_autoload_done = true; return; end
    set -g _fnva_autoload_done true

    set envs_file "$HOME/.fnva/current_envs.toml"
    if test -f "$envs_file"; and command -v fnva >/dev/null 2>&1
        set -l _restored
        for line in (cat "$envs_file")
            # ... 解析 TOML 行 ...
            set _t (mktemp)
            _FNVA_QUIET=1 command fnva $key use $value > $_t 2>/dev/null
            source $_t >/dev/null 2>&1
            rm -f $_t
            set -e _FNVA_QUIET
            set -a _restored $value
        end
        if test (count $_restored) -gt 0
            echo "[fnva] restored: "(string join ' ' $_restored)
        end
    end
end
```

### PowerShell 实现

```powershell
$fnvaAutoLoadDone = $false
function fnva-AutoLoadDefault {
    if ($fnvaAutoLoadDone) { return }
    $fnvaAutoLoadDone = $true

    $envsFile = "$env:USERPROFILE\.fnva\current_envs.toml"
    if ((Test-Path $envsFile) -and (Get-Command fnva -ErrorAction SilentlyContinue)) {
        $restored = @()
        foreach ($line in $lines) {
            # ... 解析 TOML 行 ...
            $env:_FNVA_QUIET = "1"
            $envScript = (& fnva.cmd $key use $value 2>$null) -join "`n"
            if ($envScript) { Invoke-Expression $envScript; $restored += $value }
            Remove-Item Env:\_FNVA_QUIET
        }
        if ($restored.Count -gt 0) {
            Write-Host "[fnva] restored: $($restored -join ' ')" -ForegroundColor DarkGray
        }
    }
}
```

### `_FNVA_QUIET` 机制

Autoload 必须静默执行，不能像交互式切换那样输出详细信息。

**生命周期**:
1. `env_script=$(_FNVA_QUIET=1 command fnva ...)` — `_FNVA_QUIET=1` 仅在 fnva 进程内生效
2. `_FNVA_QUIET=1 eval "$env_script"` — eval 执行期间，脚本中的 `if [[ -z "$_FNVA_QUIET" ]]` 为假，echo 跳过
3. `unset _FNVA_QUIET` — 清除，确保不影响后续操作

**常见错误**: 在 eval 之前 unset `_FNVA_QUIET`，导致 eval 时变量不存在，echo 语句全部执行。

### 防重复执行

通过 `_fnva_autoload_done` 标志防止多次执行（例如 `.bashrc` 被 source 多次）。

## Wrapper 函数

### 职责

拦截 `fnva java/llm/cc use <name>` 命令，将 fnva 输出的脚本在当前 shell 中执行，
使环境变量在当前终端会话生效。

### Bash/Zsh Wrapper

```bash
fnva() {
    if [[ $# -ge 2 && ("$1" == "java" || "$1" == "llm" || "$1" == "cc") && "$2" == "use" ]]; then
        local temp_file
        temp_file="$(mktemp)"
        command fnva "$@" > "$temp_file"   ← 捕获脚本到临时文件
        source "$temp_file"                 ← 在当前 shell 执行
        rm -f "$temp_file"                  ← 清理
    else
        command fnva "$@"                   ← 非切换命令直接透传
    fi
}
```

### Fish Wrapper

```fish
function fnva
    if test (count $argv) -ge 2; and string match -q -r "^(java|llm|cc)$" $argv[1]; and test $argv[2] = "use"
        set temp_file (mktemp)
        command fnva $argv > $temp_file
        source $temp_file
        rm -f $temp_file
    else
        command fnva $argv
    end
end
```

### PowerShell Wrapper

```powershell
function fnva {
    if ($args.Count -ge 2 -and ($args[0] -in @("java","llm","cc")) -and $args[1] -eq "use") {
        $script = & fnva.cmd @args 2>$null
        if ($script) { Invoke-Expression ($script -join "`n") }
    } else {
        & fnva.cmd @args
    }
}
```

### `command fnva` vs `fnva`

Wrapper 函数内部必须使用 `command fnva` 调用外部命令，避免无限递归：

```
fnva cc use glmcc          ← 用户调用，触发 wrapper
  └─ command fnva cc use   ← "command" 确保调用外部二进制，不触发 wrapper
       └─ fnva (Rust)      ← 输出脚本到 stdout
```

如果误用 `fnva` 而非 `command fnva`：
```
fnva cc use glmcc          ← 用户调用，触发 wrapper
  └─ fnva cc use glmcc     ← 再次触发 wrapper → 无限递归！
```

## current_envs.toml

### 格式

```toml
cc = "glmcc"
java = "17"
llm = "anthropic"
```

键名与 `EnvironmentType` 枚举对应（小写）：
- `java` → `EnvironmentType::Java`
- `llm` → `EnvironmentType::Llm`
- `cc` → `EnvironmentType::Cc`

### 读写

| 操作 | 方法 | 文件 |
|------|------|------|
| 读取 | `CurrentEnvs::read()` | `src/infrastructure/shell/current_envs.rs:32` |
| 写入 | `CurrentEnvs::write(env_type, name)` | 同上 `:44` |
| 清除 | `CurrentEnvs::clear(env_type)` | 同上 `:51` |

### Autoload 解析方式

Autoload 不使用 TOML 解析器，而是直接逐行读取：

```bash
while IFS='=' read -r key value; do
    key=$(echo "$key" | tr -d '[:space:]')
    value=$(echo "$value" | tr -d '[:space:]' | tr -d '"')
```

这意味着 TOML 文件中不能有注释、空行、或复杂格式（如数组）。

## Shell 集成安装位置

| Shell | 配置文件 | 安装方式 |
|-------|---------|---------|
| Bash | `~/.bashrc` | `eval "$(fnva env --shell bash)"` |
| Zsh | `~/.zshrc` | `eval "$(fnva env --shell bash)"` |
| Fish | `~/.config/fish/config.fish` | `fnva env --shell fish | source` |
| PowerShell | `$PROFILE` | `fnva env --shell powershell \| Out-String \| Invoke-Expression` |
| CMD | 注册表 (AutoRun) | 自动安装 |

### 安装标记

Shell 配置文件中的 fnva 代码带有标记：
```
# fnva auto integration (added by npm install)
```
卸载脚本根据此标记清理。

## 跨平台差异总结

| 特性 | Bash/Zsh | Fish | PowerShell |
|------|----------|------|------------|
| Wrapper 机制 | temp_file + source | temp_file + source | Invoke-Expression |
| Autoload 静默 | `_FNVA_QUIET=1 eval` | `_FNVA_QUIET=1 eval` | `$env:_FNVA_QUIET` + Remove-Item |
| 防递归 | `command fnva` | `command fnva` | `fnva.cmd` |
| TOML 解析 | `while IFS='=' read` | `string match -r` | `-match` 正则 |
| 汇总输出 | `echo "[fnva] restored:..."` | `echo "[fnva] restored:..."` | `Write-Host ... -ForegroundColor DarkGray` |

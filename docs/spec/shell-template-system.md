# Shell 模板系统规格

## 概述

fnva 通过生成 shell 脚本来设置环境变量。脚本内容由 Handlebars 模板引擎渲染，
模板定义在 Rust 源码中作为内联字符串常量。

## 双模板体系

项目中存在两套模板文件：

| 体系 | 位置 | 是否运行时使用 |
|------|------|---------------|
| Rust 内联模板 | `src/infrastructure/shell/script_strategy.rs` 中的 `const` 字符串 | **是** |
| Handlebars 参考文件 | `src/infrastructure/shell/templates/*.hbs` | **否**（仅作参考） |

**关键规则**: Rust 代码通过 `register_template_string()` 注册内联模板，不读取 `.hbs` 文件。
修改时必须**同步更新两边**，否则 `.hbs` 文件会与实际行为不一致。

## 12 个模板清单

### Switch 模板（环境切换脚本）

每个 shell 类型有 2 个 switch 模板（Java / LLM+CC），共 8 个：

| 模板常量名 | 注册名 | 行号 | 用途 |
|-----------|--------|------|------|
| `POWERSHELL_JAVA_SWITCH_TEMPLATE` | `powershell_java_switch` | 388-415 | PowerShell Java 切换 |
| `POWERSHELL_LLM_SWITCH_TEMPLATE` | `powershell_llm_switch` | 546-597 | PowerShell LLM/CC 切换 |
| `BASH_JAVA_SWITCH_TEMPLATE` | `bash_java_switch` | 469-496 | Bash/Zsh Java 切换 |
| `BASH_LLM_SWITCH_TEMPLATE` | `bash_llm_switch` | 599-647 | Bash/Zsh LLM/CC 切换 |
| `FISH_JAVA_SWITCH_TEMPLATE` | `fish_java_switch` | 649-675 | Fish Java 切换 |
| `FISH_LLM_SWITCH_TEMPLATE` | `fish_llm_switch` | 719-766 | Fish LLM/CC 切换 |
| `CMD_JAVA_SWITCH_TEMPLATE` | `cmd_java_switch` | 768-795 | CMD Java 切换 |
| `CMD_LLM_SWITCH_TEMPLATE` | `cmd_llm_switch` | 824-873 | CMD LLM/CC 切换 |

**模板选择逻辑**: 各 Strategy 的 `generate_switch_script()` 方法根据 `EnvironmentType` 选择：
- `Java` → `*_java_switch`
- `Llm` 或 `Cc` → `*_llm_switch`（CC 和 LLM 共用同一模板，通过 `{{env_type}}` 区分）

### Integration 模板（Shell 集成脚本）

每个 shell 类型有 1 个 integration 模板，共 4 个：

| 模板常量名 | 注册名 | 行号 | 安装方式 |
|-----------|--------|------|---------|
| `POWERSHELL_INTEGRATION_TEMPLATE` | `powershell_integration` | 417-467 | `$PROFILE` |
| `BASH_INTEGRATION_TEMPLATE` | `bash_integration` | 498-543 | `~/.bashrc` / `~/.zshrc` |
| `FISH_INTEGRATION_TEMPLATE` | `fish_integration` | 677-717 | `~/.config/fish/config.fish` |
| `CMD_INTEGRATION_TEMPLATE` | `cmd_integration` | 797-822 | 注册表 |

## Handlebars 变量参考

### Java Switch 模板变量

数据来源: `src/environments/java/environment_manager.rs` `use_env()` 方法

| 变量 | 示例值 | 说明 |
|------|--------|------|
| `{{env_name}}` | `"17"` | 环境名称 |
| `{{env_type}}` | `"Java"` | 环境类型 |
| `{{java_home}}` | `"/Users/user/.fnva/java-packages/17/Contents/Home"` | JDK 路径 |
| `{{java_bin}}` | `"/Users/user/.fnva/java-packages/17/Contents/Home/bin"` | JDK bin 路径 |

Java 模板数据构建 (`environment_manager.rs:419`):
```rust
let config = serde_json::json!({ "java_home": java_installation.java_home });
```
然后 `script_strategy.rs` 的 `generate_switch_script` 会添加 `java_bin`、`env_name`、`env_type`。

### LLM/CC Switch 模板变量

数据来源:
- CC: `src/environments/cc/environment_manager.rs` `use_env()` 方法 (line 196)
- LLM: `src/environments/llm/environment_manager.rs` `use_env()` 方法 (line 201)

| 变量 | 来源 | 说明 |
|------|------|------|
| `{{env_name}}` | 切换时传入 | 环境名称 |
| `{{env_type}}` | `Cc` 或 `Llm` | 区分 Claude Code 和通用 LLM |
| `{{config.anthropic_auth_token}}` | CC/LLM manager | Anthropic API Token（已解析） |
| `{{config.anthropic_base_url}}` | CC/LLM manager | Anthropic API Base URL（已解析） |
| `{{config.opus_model}}` | CC manager | Opus 模型名 |
| `{{config.sonnet_model}}` | CC manager | Sonnet 模型名 |
| `{{config.haiku_model}}` | CC manager | Haiku 模型名 |
| `{{config.default_model}}` | CC manager | 默认模型（同 sonnet_model） |
| `{{config.api_key}}` | LLM/CC config | 通用 API Key |
| `{{config.base_url}}` | LLM/CC config | 通用 Base URL |
| `{{config.model}}` | LLM config | LLM 模型名 |

### Handlebars 辅助函数

在 `script_strategy.rs` 中注册的自定义 helper：

| Helper | 用途 | 使用场景 |
|--------|------|---------|
| `escape_backslash` | 反斜杠转义 | Windows 路径（PowerShell/CMD） |
| `to_upper` | 转大写 | 环境变量名 |
| `path_join` | 路径拼接 | 跨平台路径 |
| `env_var_name` | 环境变量名生成 | Shell 差异化 |

### 条件渲染

LLM/CC 模板使用 Handlebars 条件块区分行为：

```handlebars
{{#if (eq env_type "Cc")}}Claude Code (CC){{else}}LLM{{/if}}
```

以及字段存在性检查：

```handlebars
{{#if config.anthropic_auth_token}}
export ANTHROPIC_AUTH_TOKEN="{{config.anthropic_auth_token}}"
{{/if}}
```

## `_FNVA_QUIET` 生命周期

`_FNVA_QUIET` 控制环境切换时的 echo 输出。**仅在 autoload 时设为 `1`，交互式使用时不设置。**

### 正确的生命周期（v0.0.69+）

```
autoload 函数启动
  └─ 读取 current_envs.toml
      └─ for each env:
          ├─ _FNVA_QUIET=1 eval "$(command fnva "$key" use "$value")" >/dev/null 2>&1
          │   ├─ $(command fnva ...) 捕获脚本（不创建中间变量）
          │   ├─ _FNVA_QUIET=1 eval 在 eval 期间设置变量
          │   └─ >/dev/null 2>&1 彻底抑制所有输出（包括 zsh 变量打印）
          │
          ├─ unset _FNVA_QUIET
          │   └─ 清除，不影响后续环境切换
          │
          └─ _restored="$_restored $value"
              └─ 记录已恢复的环境名
  └─ echo "[fnva] restored:$_restored"  ← 唯一的输出
```

**设计要点**:
- 使用 `eval "$(command fnva ...)"` 内联写法，不创建任何中间变量（`env_script`、`_t` 等）
- zsh 会打印所有 `var=$(command)` 赋值语句，内联写法避免了这个问题
- Fish 不支持 `eval "$()"` 语法，使用 temp_file + source 方式代替

### 历史缺陷

**v0.0.62 及更早** — `_FNVA_QUIET` 在 eval 前被清除：
```
env_script=$(_FNVA_QUIET=1 command fnva ...)  ← _FNVA_QUIET 只在 fnva 进程内生效
eval "$env_script"                             ← eval 时 _FNVA_QUIET 已不存在！
```

**v0.0.65-0.0.66** — `_FNVA_QUIET` 保留到 eval，但 zsh 打印变量赋值：
```
env_script=$(_FNVA_QUIET=1 command fnva ...)   ← zsh 打印 env_script=$'...'
_FNVA_QUIET=1 eval "$env_script" >/dev/null    ← >/dev/null 无法阻止变量赋值打印
```

**v0.0.67** — 尝试 temp_file 方案，但 zsh 同样打印 `_t=...`：
```
local _t; _t="$(mktemp)"                       ← zsh 打印 _t=/var/folders/...
command fnva ... > "$_t"; source "$_t"
```

### 各 Shell 的 `_FNVA_QUIET` 实现

| Shell | 设置方式 | 检查方式 | 清除方式 |
|-------|---------|---------|---------|
| Bash/Zsh | `_FNVA_QUIET=1 eval "$(command fnva ...)" >/dev/null 2>&1` | `if [[ -z "$_FNVA_QUIET" ]]` | `unset _FNVA_QUIET` |
| Fish | temp_file + `source $_t >/dev/null 2>&1` | `if not set -q _FNVA_QUIET` | `set -e _FNVA_QUIET` |
| PowerShell | `$env:_FNVA_QUIET = "1"` (在 Invoke-Expression 前) | `if (-not $env:_FNVA_QUIET)` | `Remove-Item Env:\_FNVA_QUIET` (在 Invoke-Expression 后) |

## Shell 语法差异对照

| 操作 | Bash/Zsh | Fish | PowerShell | CMD |
|------|----------|------|------------|-----|
| 导出变量 | `export VAR="val"` | `set -gx VAR "val"` | `$env:VAR = "val"` | `set VAR=val` |
| 设置 PATH | `export PATH="bin:$PATH"` | `set -gx PATH "bin" $PATH` | `$env:PATH = "bin;" + $env:PATH` | `set PATH=bin;%PATH%` |
| 条件判断 | `if [[ -z "$VAR" ]]` | `if not set -q VAR` | `if (-not $env:VAR)` | 无（CMD 不支持） |
| 静默执行 | `VAR=1 eval "$script"` | `VAR=1 eval "$script"` | `$env:VAR = "1"; Invoke-Expression` | N/A |
| 命令替换 | `var=$(cmd)` | `set var (cmd)` | `$var = (& cmd)` | `for /f ...` |

## 已知陷阱

### 1. Zsh 打印所有变量赋值

zsh 在某些配置下会打印 `var=$(command)` 形式的赋值语句。不论变量名是 `env_script`、`_t` 还是其他名称，都会被打印。

**影响版本**: v0.0.65-0.0.67 的 autoload 使用了中间变量，在 zsh 下产生噪音输出。

**最终解决方案（v0.0.69+）**: 使用 `eval "$(command fnva ...)"` 内联写法，不创建任何中间变量，从根源消除问题。

**排查方法**: 如果仍有变量打印，检查 `set -x` / `setopt VERBOSE` 是否开启。

### 2. `_FNVA_QUIET=1 eval` 在不同 Shell 的行为

- **Bash**: `VAR=val eval 'script'` — VAR 在 eval 执行期间可见 ✓
- **Zsh**: 同 Bash ✓，但某些版本在 `set -x` 下可能打印变量赋值
- **Fish**: `VAR=1 eval "$script"` — 需验证 Fish 是否支持此语法
- **PowerShell**: 不支持前缀变量语法，必须在 eval 前显式设置环境变量

### 3. `.hbs` 文件与内联模板不一致

`.hbs` 文件在 `templates/` 目录下，但不被 Rust 代码加载。修改模板时如果只改了 `.hbs`
而没改 Rust 内联常量（或反之），会导致参考文档与实际行为不一致。

### 4. Integration 模板中的 `command fnva`

Autoload 在 wrapper 函数定义之前运行，因此 autoload 内的 `fnva` 调用自然走外部命令。
但为防御性编程，仍使用 `command fnva` 确保不触发 wrapper。

### 5. 集成命令是 `fnva env env`（两个 env）

正确的集成加载命令是 `eval "$(fnva env env --shell bash)"`，不是 `fnva env --shell bash`。
第一个 `env` 是顶级子命令，第二个 `env` 是 `GenerateEnv` 子命令。

Autoload 在 wrapper 函数定义之前运行，因此 autoload 内的 `fnva` 调用自然走外部命令。
但为防御性编程，仍使用 `command fnva` 确保不触发 wrapper。

### 5. Switch 模板中的环境变量转义

模板直接将 API Key 等值嵌入 shell 脚本字符串。如果值包含双引号或特殊字符，
可能导致脚本语法错误。当前未做转义处理。

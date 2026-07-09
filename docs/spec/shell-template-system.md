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

## 16 个模板清单

### Switch 模板（环境切换脚本）

每个 shell 类型有 3 个 switch 模板（Java / Maven / CC），共 12 个：

| 模板常量名 | 注册名 | 用途 |
|-----------|--------|------|
| `POWERSHELL_JAVA_SWITCH_TEMPLATE` | `powershell_java_switch` | PowerShell Java 切换 |
| `POWERSHELL_MAVEN_SWITCH_TEMPLATE` | `powershell_maven_switch` | PowerShell Maven 切换 |
| `POWERSHELL_CC_SWITCH_TEMPLATE` | `powershell_cc_switch` | PowerShell Claude Code 切换 |
| `BASH_JAVA_SWITCH_TEMPLATE` | `bash_java_switch` | Bash/Zsh Java 切换 |
| `BASH_MAVEN_SWITCH_TEMPLATE` | `bash_maven_switch` | Bash/Zsh Maven 切换 |
| `BASH_CC_SWITCH_TEMPLATE` | `bash_cc_switch` | Bash/Zsh Claude Code 切换 |
| `FISH_JAVA_SWITCH_TEMPLATE` | `fish_java_switch` | Fish Java 切换 |
| `FISH_MAVEN_SWITCH_TEMPLATE` | `fish_maven_switch` | Fish Maven 切换 |
| `FISH_CC_SWITCH_TEMPLATE` | `fish_cc_switch` | Fish Claude Code 切换 |
| `CMD_JAVA_SWITCH_TEMPLATE` | `cmd_java_switch` | CMD Java 切换 |
| `CMD_MAVEN_SWITCH_TEMPLATE` | `cmd_maven_switch` | CMD Maven 切换 |
| `CMD_CC_SWITCH_TEMPLATE` | `cmd_cc_switch` | CMD Claude Code 切换 |

**模板选择逻辑**: 各 Strategy 的 `generate_switch_script()` 方法根据 `EnvironmentType` 选择：
- `Java` → `*_java_switch`
- `Maven` → `*_maven_switch`
- `Cc` → `*_cc_switch`

### Integration 模板（Shell 集成脚本）

每个 shell 类型有 1 个 integration 模板，共 4 个：

| 模板常量名 | 注册名 | 安装方式 |
|-----------|--------|---------|
| `POWERSHELL_INTEGRATION_TEMPLATE` | `powershell_integration` | `$PROFILE` |
| `BASH_INTEGRATION_TEMPLATE` | `bash_integration` | `~/.bashrc` / `~/.zshrc` |
| `FISH_INTEGRATION_TEMPLATE` | `fish_integration` | `~/.config/fish/config.fish` |
| `CMD_INTEGRATION_TEMPLATE` | `cmd_integration` | 注册表 |

## Handlebars 变量参考

### Java Switch 模板变量

数据来源: `src/environments/java/environment_manager.rs` 中的 `use_env()` 方法（并在 `PowerShellStrategy`/`BashStrategy` 等 `generate_switch_script` 里加工补充 `java_bin` 等）：

| 变量 | 示例值 | 说明 |
|------|--------|------|
| `{{env_name}}` | `"17"` | 环境名称 |
| `{{env_type}}` | `"Java"` | 环境类型 |
| `{{java_home}}` | `"/home/user/.fnva/packages/java/17"` | JDK 路径 |
| `{{java_bin}}` | `"/home/user/.fnva/packages/java/17/bin"` | JDK bin 路径 |

### Maven Switch 模板变量

数据来源: `src/environments/maven/environment_manager.rs` 中的 `use_env()` 方法：

| 变量 | 示例值 | 说明 |
|------|--------|------|
| `{{env_name}}` | `"3.9.16"` | 环境名称 |
| `{{env_type}}` | `"Maven"` | 环境类型 |
| `{{maven_home}}` | `"/home/user/.fnva/packages/maven/3.9.16"` | Maven 安装路径 |
| `{{maven_bin}}` | `"/home/user/.fnva/packages/maven/3.9.16/bin"` | Maven bin 路径 |

### CC (Claude Code) Switch 模板变量

数据来源: `src/environments/cc/environment_manager.rs` 中的 `use_env()` 方法：

| 变量 | 说明 |
|------|------|
| `{{env_name}}` | 环境名称 |
| `{{env_type}}` | 环境类型（固定为 `"Cc"` 或 `"CC"`） |
| `{{config.api_key}}` | 通用 API Key |
| `{{config.base_url}}` | 通用 Base URL |
| `{{config.sonnet_model}}` | Sonnet 模型名称 |
| `{{config.anthropic_auth_token}}` | Anthropic API Token（由 api_key 解析） |
| `{{config.anthropic_base_url}}` | Anthropic API Base URL（由 base_url 解析） |
| `{{config.opus_model}}` | Opus 模型名 |
| `{{config.haiku_model}}` | Haiku 模型名 |
| `{{config.default_model}}` | 默认模型名（同 sonnet_model） |

### Handlebars 辅助函数

在 `script_strategy.rs` 中注册的自定义 helper：

| Helper | 用途 | 使用场景 |
|--------|------|---------|
| `escape_backslash` | 反斜杠转义 | Windows 路径（PowerShell/CMD） |
| `to_upper` | 转大写 | 环境变量名 |
| `path_join` | 路径拼接 | 跨平台路径 |
| `env_var_name` | 环境变量名生成 | Shell 差异化 |

### 条件渲染

CC 模板使用 Handlebars 条件块检查字段存在性：

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
  └─ 读取 state/current_envs.toml
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

zsh 在某些配置下会打印 `var=$(command)` 形式 of 赋值语句。不论变量名是 `env_script`、`_t` 还是其他名称，都会被打印。

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

### 5. Switch 模板中的环境变量转义

模板直接将 API Key 等值嵌入 shell 脚本字符串。如果值包含双引号或特殊字符，
可能导致脚本语法错误。当前未做转义处理。

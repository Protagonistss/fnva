# 环境切换流程规格

## 概述

描述从用户输入 `fnva cc use glmcc` 到环境变量在终端生效的完整链路。
分为**交互式切换**和 **autoload 启动恢复**两条路径。

## 交互式切换流程

### 1. 命令解析

```
用户输入: fnva cc use glmcc
    │
    ▼
bin/fnva.js (Node.js 入口)
    │
    ├─ 定位 Rust 二进制 (buildBinaryPath)
    │   ├─ platforms/<os>-<arch>/fnva[.exe]  (npm 包)
    │   ├─ $FNVA_NATIVE_PATH                 (环境变量覆盖)
    │   └─ target/{release,debug}/fnva       (本地构建)
    │
    ├─ Unix: stdio: 'inherit' 透传 (line 492-508)
    │   └─ shell wrapper 函数负责捕获输出
    │
    └─ Windows: 捕获 stdout (line 511-638)
        └─ 解析输出，生成 PowerShell 脚本
```

### 2. Rust 命令处理

```
fnva cc use glmcc
    │
    ▼
src/cli/commands.rs — clap 命令定义
    │  Cc 子命令 → Use { name: "glmcc" }
    ▼
src/cli/handlers.rs — handle_cc_command() (line 367)
    │  调用 switcher.switch_environment(Cc, "glmcc", shell_type)
    ▼
src/core/switcher.rs — switch_environment() (line 51)
    │
    ├─ 1. 获取环境管理器 (registry.get(EnvironmentType::Cc))
    ├─ 2. 验证环境存在
    ├─ 3. 生成切换脚本
    │      manager.use_env("glmcc", shell_type)
    │      └─ 构建配置 JSON → 选择模板策略 → 渲染 Handlebars
    ├─ 4. 更新会话状态 (session_manager.update)
    ├─ 5. 持久化到 current_envs.toml
    │      cc = "glmcc"
    └─ 6. 记录历史
```

### 3. 脚本生成

```
src/environments/cc/environment_manager.rs — use_env() (line 196)
    │
    ├─ 构建 JSON 数据:
    │   {
    │     "anthropic_auth_token": "1c1f8da9...",
    │     "anthropic_base_url": "https://open.bigmodel.cn/api/anthropic",
    │     "opus_model": "glm-5.1",
    │     "sonnet_model": "glm-5.1",
    │     "haiku_model": "glm-5.1"
    │   }
    │
    └─ 调用 script_generator.generate()
        │
        ▼
src/infrastructure/shell/script_factory.rs
    │  根据 shell_type 选择策略
    │  Bash → BashScriptStrategy
    │  PowerShell → PowerShellScriptStrategy
    │  Fish → FishScriptStrategy
    │  CMD → CmdScriptStrategy
    ▼
src/infrastructure/shell/script_strategy.rs
    │  EnvironmentType::Cc + Bash → "bash_llm_switch" 模板
    │  Handlebars 渲染 → 输出 shell 脚本字符串
    ▼
stdout 输出:
    #!/bin/bash
    export ANTHROPIC_AUTH_TOKEN="..."
    export ANTHROPIC_BASE_URL="..."
    export ANTHROPIC_DEFAULT_SONNET_MODEL="..."
    ...
    if [[ -z "$_FNVA_QUIET" ]]; then
        echo "[OK] Switched to Claude Code (CC) environment: glmcc"
        echo "[KEY] Anthropic Auth Token: [SET]"
        echo "[URL] Base URL: https://open.bigmodel.cn/api/anthropic"
    fi
```

### 4. Shell 执行

#### Unix (Bash/Zsh) 路径

```
用户输入: fnva cc use glmcc
    │
    ▼  shell profile 中定义的 wrapper 函数拦截
fnva() {
    if [[ "$1" == "cc" && "$2" == "use" ]]; then
        temp_file=$(mktemp)
        command fnva "$@" > "$temp_file"    ← 捕获脚本到临时文件
        source "$temp_file"                 ← 在当前 shell 中执行
        rm -f "$temp_file"                  ← 清理
    fi
}
    │
    ▼  source 执行脚本
    ├─ export ANTHROPIC_AUTH_TOKEN="..."    ← 环境变量在当前 shell 生效
    ├─ export ANTHROPIC_BASE_URL="..."
    └─ echo "[OK] Switched to..."           ← 输出确认信息
```

关键: `_FNVA_QUIET` 未设置（交互式），所以 echo 语句正常输出。

#### Windows (PowerShell) 路径

```
用户输入: fnva cc use glmcc
    │
    ▼  bin/fnva.js 捕获模式 (line 511+)
    ├─ spawnSync(binaryPath, args) 捕获 stdout
    ├─ 检测输出包含 ANTHROPIC_ 等关键字
    ├─ 生成 PowerShell 脚本 (generateSimpleScript)
    │
    ▼  fnva PowerShell wrapper 函数
    ├─ 捕获输出到 temp script
    ├─ Invoke-Expression 执行
    └─ 输出确认信息
```

### 完整时序

```
┌──────┐   ┌──────────┐   ┌──────────┐   ┌──────────────┐   ┌───────────┐
│ User │   │ Shell    │   │ fnva.js  │   │ fnva (Rust)  │   │ Handlebars│
└──┬───┘   └────┬─────┘   └────┬─────┘   └──────┬───────┘   └─────┬─────┘
   │            │              │                 │                  │
   │ fnva cc   │              │                 │                  │
   │ use glmcc │              │                 │                  │
   │──────────▶│              │                 │                  │
   │            │ wrapper 拦截  │                 │                  │
   │            │──────────────▶│                 │                  │
   │            │              │ stdio:inherit   │                  │
   │            │              │────────────────▶│                  │
   │            │              │                 │ 解析命令          │
   │            │              │                 │─────────────────▶│
   │            │              │                 │  渲染模板         │
   │            │              │                 │◀─────────────────│
   │            │              │  脚本字符串      │                  │
   │            │              │◀────────────────│                  │
   │            │ 捕获到文件    │                 │                  │
   │            │◀─────────────│                 │                  │
   │            │ source 执行   │                 │                  │
   │            │──────┐       │                 │                  │
   │            │      │ export ANTHROPIC_*      │                  │
   │            │◀─────┘       │                 │                  │
   │ [OK] ...  │              │                 │                  │
   │◀──────────│              │                 │                  │
```

## Autoload 启动恢复流程

```
Shell 启动
    │
    ▼  加载 .zshrc / .bashrc / $PROFILE
eval "$(fnva env env --shell bash)"
    │
    ▼  输出 integration script 并执行
    │
    ├─ 定义 fnva_autoload_default() 函数
    ├─ fnva_autoload_default  ← 立即调用（此时 wrapper 尚未定义）
    │   │
    │   ▼  读取 ~/.fnva/current_envs.toml
    │   │  cc = "glmcc"
    │   │  java = "17"
    │   │
    │   ├─ for cc = "glmcc":
    │   │   _FNVA_QUIET=1 eval "$(command fnva cc use glmcc)" >/dev/null 2>&1
    │   │   unset _FNVA_QUIET
    │   │
    │   ├─ for java = "17":
    │   │   _FNVA_QUIET=1 eval "$(command fnva java use 17)" >/dev/null 2>&1
    │   │   unset _FNVA_QUIET
    │   │
    │   └─ echo "[fnva] restored: glmcc 17"  ← 唯一输出
    │
    └─ 定义 fnva() wrapper 函数  ← autoload 之后才定义
```

## 关键设计决策

### 为什么 Unix 用 temp_file + source？

Shell 函数无法通过 stdout 返回值给调用者。`fnva cc use glmcc` 需要在**当前 shell** 设置环境变量，
但 `fnva` 是外部命令，其 `export` 只影响子进程。解决方案：

1. 外部命令输出脚本到 stdout
2. Wrapper 函数捕获到 temp file
3. `source` 在当前 shell 执行

### 为什么 autoload 需要 `_FNVA_QUIET`？

Autoload 在每次新终端启动时运行。如果不静默，用户每次开终端都会看到一堆切换消息。
`_FNVA_QUIET` 让 switch 脚本中的 echo 跳过，只输出一行汇总。

### 为什么 autoload 在 wrapper 之前运行？

Integration script 的顺序是：
1. `fnva_autoload_default()` 函数定义
2. `fnva_autoload_default` 调用
3. `fnva()` wrapper 定义

这样 autoload 调用的 `fnva` 直接走外部命令（因为 wrapper 还不存在），
避免 wrapper 的 temp_file 逻辑干扰 autoload 的 `env_script=$()` 捕获。

### 为什么 Windows 不用 temp_file？

PowerShell 通过 `Invoke-Expression` 在当前作用域执行脚本字符串，
不需要像 Bash 那样通过文件中转。`fnva.js` 直接将脚本字符串传给 wrapper。

## current_envs.toml

路径: `~/.fnva/current_envs.toml`

格式:
```toml
cc = "glmcc"
java = "17"
llm = "anthropic"
```

- 写入: `src/infrastructure/shell/current_envs.rs` → `write(env_type, name)`
- 读取: autoload 通过 `while IFS='=' read -r key value` 逐行解析
- 每次执行 `fnva <type> use <name>` 时更新对应行

## 数据流总结

```
config.toml ──读取──▶ EnvironmentManager
                          │
                          ▼
                      use_env() ──构建──▶ JSON config
                                         │
switcher.rs ──调用──▶ script_factory ──选择──▶ ScriptStrategy
                                         │
                                    Handlebars 渲染
                                         │
                                         ▼
                                    Shell 脚本字符串
                                         │
                   stdout ◀── fnva.js ◀──┘
                      │
                      ▼
              wrapper 函数捕获
                      │
                      ▼
              source / Invoke-Expression
                      │
                      ▼
              环境变量在当前 shell 生效
                      │
                      ▼
              current_envs.toml 更新（持久化）
```

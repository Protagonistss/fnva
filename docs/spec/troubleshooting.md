# 故障排查手册

## 已知问题与解决方案

### 1. Autoload 输出噪音（Mac/Linux 终端启动时输出过多）

**症状**: 每次打开新终端，显示多行切换信息，而不是仅一行 `[fnva] restored: glmcc 17`。

**根因**: `_FNVA_QUIET` 在 `eval` 执行 switch 脚本时已不存在。

**检查方法**:
```bash
# 查看当前 integration script
fnva env --shell bash

# 检查 autoload 中 _FNVA_QUIET 的位置
fnva env --shell bash | grep -A3 "eval.*env_script"
```

**正确行为**: `_FNVA_QUIET=1 eval "$env_script"` 然后 `unset _FNVA_QUIET`。
**错误行为**: `eval "$env_script"` 之前已 `unset _FNVA_QUIET`（或从未在 eval 时设置）。

**修复**: 确保 Rust 内联模板中 autoload 的 eval 行有 `_FNVA_QUIET=1` 前缀。

### 2. Zsh 中 `env_script=$'...'` 被打印

**症状**: 终端启动时输出 `env_script=$'\n#!/bin/bash\n...'`。

**根因**: Zsh 在某些配置下会打印变量赋值：
- `set -x` (xtrace) 开启
- `setopt VERBOSE` 开启
- 终端调试插件副作用

**检查方法**:
```bash
# 检查是否开启了调试选项
echo $options[xtrace]
echo $options[verbose]

# 临时关闭
set +x
unsetopt VERBOSE
```

**缓解方案**: 如果无法关闭 shell 调试选项，可以在 autoload 的 eval 中重定向输出：
```bash
eval "$env_script" >/dev/null 2>&1
```

### 3. Wrapper 函数无限递归

**症状**: 运行 `fnva cc use glmcc` 时 shell 卡住或报 "command not found"。

**根因**: Wrapper 函数内部使用 `fnva` 而非 `command fnva`。

**检查方法**:
```bash
# 查看 wrapper 定义
type fnva

# 确认内部使用 command fnva
type fnva | grep "command fnva"
```

**正确**: wrapper 内部使用 `command fnva "$@"`
**错误**: wrapper 内部使用 `fnva "$@"`（递归调用自身）

### 4. 环境变量未生效

**症状**: `fnva cc use glmcc` 显示成功，但 `echo $ANTHROPIC_AUTH_TOKEN` 为空。

**可能原因**:

1. **Integration script 未安装**: wrapper 函数不存在，fnva 的输出直接打印到终端而非被执行。
   ```bash
   # 检查
   type fnva
   # 如果显示 "fnva is /usr/local/bin/fnva" 而非函数，说明未安装 integration
   ```

2. **Shell 类型不匹配**: `.zshrc` 中使用了 bash 格式的 integration script。
   ```bash
   # 重新安装
   echo 'eval "$(fnva env --shell bash)"' >> ~/.zshrc
   ```

3. **fnva.js 版本与 Rust 二进制不一致**: npm 包中的 fnva.js 与平台二进制版本不匹配。
   ```bash
   fnva --version
   ```

### 5. Windows PowerShell 不生效

**症状**: `fnva java use 17` 在 PowerShell 中不切换环境。

**常见原因**:

1. **`$PROFILE` 未加载**: 检查执行策略。
   ```powershell
   Get-ExecutionPolicy
   # 应为 RemoteSigned 或 Unrestricted
   Set-ExecutionPolicy RemoteSigned -Scope CurrentUser
   ```

2. **fnva.cmd 不在 PATH 中**: npm 全局安装路径未添加到 PATH。
   ```powershell
   Get-Command fnva
   Get-Command fnva.cmd
   ```

3. **Wrapper 使用了 fnva 而非 fnva.cmd**: PowerShell wrapper 必须调用 `fnva.cmd`。
   ```powershell
   # 查看 wrapper 定义
   Get-Content Function:\fnva
   ```

### 6. Autoload 后环境变量丢失

**症状**: 新终端中环境变量不存在，但 `[fnva] restored:...` 显示正常。

**可能原因**:

1. **current_envs.toml 格式错误**: 包含注释或空行导致解析失败。
   ```bash
   cat ~/.fnva/current_envs.toml
   # 应该只有 key = "value" 格式，无注释
   ```

2. **fnva 二进制路径变化**: autoload 运行时 fnva 不在 PATH 中。
   ```bash
   # 检查 autoload 运行时能否找到 fnva
   command -v fnva
   ```

## 诊断命令速查

```bash
# 查看 fnva 版本
fnva --version

# 查看当前环境
fnva env current

# 查看 integration script 内容
fnva env --shell bash        # Bash/Zsh
fnva env --shell fish        # Fish
fnva env --shell powershell  # PowerShell

# 查看 wrapper 函数定义
type fnva                     # Bash/Zsh
functions fnva                # Fish
Get-Content Function:\fnva    # PowerShell

# 检查 _FNVA_QUIET 状态
echo $_FNVA_QUIET             # Bash/Zsh
echo $env:_FNVA_QUIET         # PowerShell

# 查看 current_envs.toml
cat ~/.fnva/current_envs.toml

# 查看 fnva 二进制位置
command -v fnva               # Unix
Get-Command fnva              # PowerShell

# 重新安装 integration
eval "$(fnva env env --shell bash)"  # 并添加到 shell 配置文件（注意两个 env）
```

## 版本历史中的已知 Bug

| 版本 | 问题 | 修复版本 |
|------|------|---------|
| ≤0.0.62 | autoload `_FNVA_QUIET` 在 eval 前被清除，启动噪音 | 0.0.65 |
| ≤0.0.62 | `.hbs` 文件与 Rust 内联模板不同步 | 0.0.64 |
| ≤0.0.62 | autoload 使用 `fnva` 而非 `command fnva`，可能递归 | 0.0.64 |
| ≤0.0.62 | Fish autoload 未设置 `_FNVA_QUIET` | 0.0.65 |
| ≤0.0.66 | zsh 打印 `env_script` 变量赋值，`>/dev/null` 无法阻止 | 0.0.69 |
| ≤0.0.67 | zsh 打印 `_t` 变量赋值（temp_file 方案），所有中间变量都会被打印 | 0.0.69 |
| ≤0.0.68 | 集成脚本注释中命令缺少第二个 `env`（`fnva env --shell` 应为 `fnva env env --shell`） | 0.0.69 |

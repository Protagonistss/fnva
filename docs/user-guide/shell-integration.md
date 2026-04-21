# Shell 集成

fnva 支持自动配置 shell 集成，让你可以直接使用 `fnva java use 17` 这样的命令，无需手动配置。

## 安装时的自动配置

当你安装 fnva 时：

```bash
npm install -g fnva
```

fnva 会自动：
1. 检测你使用的 shell
2. 询问是否安装 shell 集成
3. 自动添加集成脚本到你的 shell 配置文件

## 支持的 Shell

| Shell | 配置文件 | 集成命令 |
|-------|----------|---------|
| Bash | `~/.bashrc` | `eval "$(fnva env env --shell bash)"` |
| Zsh | `~/.zshrc` | `eval "$(fnva env env --shell bash)"` |
| Fish | `~/.config/fish/config.fish` | `fnva env env --shell fish \| source` |
| PowerShell | `$PROFILE` | `fnva env env --shell powershell \| Out-String \| Invoke-Expression` |

注意：命令是 `fnva env env`（两个 `env`），第一个是顶级子命令，第二个是生成集成脚本的子命令。

## 手动安装

如果自动安装失败，在 shell 配置文件中添加对应的一行集成命令即可：

### Bash / Zsh

将以下内容添加到 `~/.bashrc` 或 `~/.zshrc`：

```bash
eval "$(fnva env env --shell bash)"
```

### Fish

将以下内容添加到 `~/.config/fish/config.fish`：

```fish
fnva env env --shell fish | source
```

### PowerShell

将以下内容添加到 `$PROFILE`：

```powershell
fnva env env --shell powershell | Out-String | Invoke-Expression
```

## 使用方法

安装后，你可以直接使用：

```bash
# 列出 Java 环境
fnva java list

# 切换 Java 环境
fnva java use 17

# 列出 CC 环境
fnva cc list

# 切换 CC 环境
fnva cc use glmcc

# 验证切换
java --version
```

## 工作原理

集成脚本会在 shell 启动时加载，提供两个功能：

1. **Autoload（自动恢复）**: 读取 `~/.fnva/current_envs.toml`，恢复上次使用的环境。新终端打开时只显示一行汇总，如 `[fnva] restored: glmcc 17`。
2. **Wrapper 函数**: 拦截 `fnva java/llm/cc use` 命令，将 fnva 输出的脚本在当前 shell 中执行，使环境变量在当前终端会话生效。

## 卸载

要移除 shell 集成：

1. 编辑你的 shell 配置文件（`~/.zshrc`、`~/.bashrc`、`$PROFILE` 等）
2. 删除包含 `fnva env env` 或 `fnva auto integration` 的相关行
3. 重新加载 shell 配置：`source ~/.zshrc`

## 故障排除

### 检查集成是否生效

```bash
# Bash/Zsh — 应显示 "fnva is a function"
type fnva

# Fish — 应显示函数定义
functions fnva

# PowerShell — 应显示函数内容
Get-Content Function:\fnva
```

### 重新安装集成

```bash
# 删除旧的集成脚本，重新加载
# Bash/Zsh: 从 ~/.zshrc 或 ~/.bashrc 中删除旧行，然后添加：
eval "$(fnva env env --shell bash)"
```

### 环境切换失败

1. 确认 fnva 已安装：`fnva --version`
2. 确认 wrapper 函数已加载：`type fnva`
3. 查看当前集成脚本内容：`fnva env env --shell bash`
4. 检查 current_envs.toml：`cat ~/.fnva/current_envs.toml`

### Autoload 输出噪音

如果新终端启动时显示多余的变量赋值或切换信息，说明集成脚本版本过旧。
删除 shell 配置文件中的旧集成代码，重新添加 `eval "$(fnva env env --shell bash)"` 即可。

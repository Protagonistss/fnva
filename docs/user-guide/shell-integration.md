# Shell 集成

fnva 支持自动配置 shell 集成，让你可以直接使用 `fnva java use v8` 这样的命令，无需手动配置。

## 安装时的自动配置

当你安装 fnva 时：

```bash
npm install -g fnva
```

fnva 会自动：
1. 检测你使用的 shell
2. 询问是否安装 shell 集成
3. 自动添加 fnva 函数到你的 shell 配置文件

## 支持的 Shell

| Shell | 配置文件 | 状态 |
|-------|----------|------|
| PowerShell | `$PROFILE` | ✅ 支持 |
| Bash | `~/.bashrc` | ✅ 支持 |
| Zsh | `~/.zshrc` | ✅ 支持 |
| Fish | `~/.config/fish/config.fish` | ✅ 支持 |

## 手动安装

如果你跳过了自动安装，或者想重新安装：

```bash
npm run install-shell
```

## 手动配置

如果自动安装失败，你也可以手动配置：

### PowerShell

将以下内容添加到 `$PROFILE`：

```powershell
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

### Bash/Zsh

将以下内容添加到 `~/.bashrc` 或 `~/.zshrc`：

```bash
fnva() {
    if [[ $# -ge 2 && ("$1" == "java" || "$1" == "llm" || "$1" == "cc") && "$2" == "use" ]]; then
        local temp_file=$(mktemp)
        chmod +x "$temp_file"

        FNVA_AUTO_MODE=1 fnva "$@" > "$temp_file"
        source "$temp_file"
        rm -f "$temp_file"
    else
        FNVA_AUTO_MODE=1 fnva "$@"
    fi
}
```

### Fish

将以下内容添加到 `~/.config/fish/config.fish`：

```fish
function fnva
    if test (count $argv) -ge 2; and string match -q -r "^(java|llm|cc)$" $argv[1]; and test $argv[2] = "use"
        set temp_file (mktemp)
        chmod +x $temp_file
        env FNVA_AUTO_MODE=1 fnva $argv > $temp_file
        source $temp_file
        rm -f $temp_file
    else
        env FNVA_AUTO_MODE=1 fnva $argv
    end
end
```

## 使用方法

安装后，你可以直接使用：

```bash
# 列出 Java 环境
fnva java list

# 切换 Java 环境
fnva java use v8

# 验证切换
java --version
```

## 工作原理

1. **环境检测**: fnva 检测到 `FNVA_AUTO_MODE=1` 环境变量时，会自动进入 Node.js 模式
2. **脚本生成**: 生成适合当前 shell 的环境切换脚本
3. **自动执行**: 对于环境切换命令，自动执行生成的脚本

## 禁用 Shell 集成

如果你想禁用自动安装：

```bash
# 安装时跳过 shell 集成
FNVA_SKIP_SHELL_SETUP=1 npm install -g fnva
```

## 卸载

要移除 shell 集成：

1. 编辑你的 shell 配置文件
2. 删除 `fnva 自动化函数 - 由 npm 安装自动添加` 相关的所有内容
3. 重新加载 shell 配置

## 故障排除

### PowerShell 中不生效

```powershell
# 重新加载配置
. $PROFILE

# 检查函数是否定义
Get-Command fnva
```

### Bash/Zsh 中不生效

```bash
# 重新加载配置
source ~/.bashrc  # 或 source ~/.zshrc

# 检查函数是否定义
type fnva
```

### 环境切换失败

1. 确保你使用的是 npm 版本的 fnva，而不是系统中的其他版本
2. 检查 Node.js 是否正确安装：`node --version`
3. 查看详细错误信息：`FNVA_AUTO_MODE=1 fnva java use v8`
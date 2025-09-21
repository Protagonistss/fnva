# FNM (Fast Node Manager) 离线安装脚本 - Windows 版
# 使用本地fnm.exe文件进行安装，支持用户指定安装目录和国内镜像源
# 作者: cool-utils
# 版本: 2.1

param(
    [string]$InstallPath = "",
    [string]$FnmExePath = "",
    [switch]$UseChineseMirror = $true,
    [switch]$InstallLatestNode = $false,
    [switch]$Force = $false,
    [switch]$Help = $false
)

# 显示帮助信息
if ($Help) {
    Write-Host "FNM (Fast Node Manager) 离线安装脚本" -ForegroundColor Green
    Write-Host ""
    Write-Host "用法:" -ForegroundColor Yellow
    Write-Host "  .\installer.ps1 [参数]" -ForegroundColor White
    Write-Host ""
    Write-Host "参数:" -ForegroundColor Yellow
    Write-Host "  -InstallPath <路径>     指定安装路径 (默认: C:\fnm)" -ForegroundColor White
    Write-Host "  -FnmExePath <路径>      指定fnm.exe文件路径 (默认: 脚本同目录下的fnm.exe)" -ForegroundColor White
    Write-Host "  -UseChineseMirror       使用国内镜像源 (默认: 启用)" -ForegroundColor White
    Write-Host "  -InstallLatestNode      安装完成后自动安装最新 LTS Node.js" -ForegroundColor White
    Write-Host "  -Force                  强制重新安装 (覆盖现有安装)" -ForegroundColor White
    Write-Host "  -Help                   显示此帮助信息" -ForegroundColor White
    Write-Host ""
    Write-Host "示例:" -ForegroundColor Yellow
    Write-Host "  .\installer.ps1 -InstallPath 'D:\dev\fnm'" -ForegroundColor White
    Write-Host "  .\installer.ps1 -FnmExePath 'D:\downloads\fnm.exe'" -ForegroundColor White
    Write-Host "  .\installer.ps1 -InstallPath 'C:\tools\fnm' -FnmExePath '.\my-fnm.exe'" -ForegroundColor White
    Write-Host "  .\installer.ps1 -InstallLatestNode" -ForegroundColor White
    Write-Host ""
    Write-Host "注意: 如未指定FnmExePath，脚本将使用同目录下的 fnm.exe 文件" -ForegroundColor Cyan
    exit 0
}

# 设置控制台编码为 UTF-8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

# 显示脚本标题
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "    FNM (Fast Node Manager) 离线安装脚本 v1.0" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# 检查是否已安装 FNM
function Test-FnmInstalled {
    $fnmCmd = Get-Command fnm -ErrorAction SilentlyContinue
    return $fnmCmd -ne $null
}

# 检查本地fnm.exe是否存在
function Test-LocalFnmExe {
    param([string]$CustomExePath = "")
    
    # 确定fnm.exe路径
    if ($CustomExePath) {
        # 用户指定了自定义路径
        $exePath = $CustomExePath
        # 如果是相对路径，转换为绝对路径
        if (-not [System.IO.Path]::IsPathRooted($exePath)) {
            $exePath = Join-Path (Get-Location) $exePath
        }
    } else {
        # 使用默认路径（脚本同目录下的fnm.exe）
        $scriptDir = Split-Path -Parent $MyInvocation.ScriptName
        $exePath = Join-Path $scriptDir "fnm.exe"
    }
    
    if (Test-Path $exePath) {
        $fileSize = (Get-Item $exePath).Length
        Write-Host "找到fnm.exe文件: $exePath" -ForegroundColor Green
        Write-Host "文件大小: $([math]::Round($fileSize/1MB, 2)) MB" -ForegroundColor Cyan
        
        # 验证文件是否为有效的可执行文件
        try {
            $fileInfo = Get-Item $exePath
            if ($fileInfo.Extension -eq ".exe") {
                Write-Host "文件验证通过: 有效的可执行文件" -ForegroundColor Green
                return $exePath
            } else {
                Write-Host "警告: 文件不是.exe格式" -ForegroundColor Yellow
                return $exePath
            }
        } catch {
            Write-Host "警告: 无法验证文件: $($_.Exception.Message)" -ForegroundColor Yellow
            return $exePath
        }
    } else {
        Write-Host "错误: 未找到fnm.exe文件" -ForegroundColor Red
        Write-Host "路径: $exePath" -ForegroundColor Yellow
        if ($CustomExePath) {
            Write-Host "请检查指定的fnm.exe文件路径是否正确" -ForegroundColor Yellow
        } else {
            Write-Host "请确保 fnm.exe 文件与此脚本在同一目录下，或使用 -FnmExePath 参数指定路径" -ForegroundColor Yellow
        }
        return $null
    }
}


# 检查现有安装
$fnmInstalled = Test-FnmInstalled
Write-Host "Debug: FNM 已安装: $fnmInstalled" -ForegroundColor Gray
Write-Host "Debug: Force 参数: $Force" -ForegroundColor Gray

if ($fnmInstalled -and -not $Force) {
    Write-Host "检测到 FNM 已安装!" -ForegroundColor Green
    try {
        $version = & fnm --version 2>$null
        Write-Host "当前版本: $version" -ForegroundColor Cyan
    } catch {
        Write-Host "无法获取 FNM 版本信息" -ForegroundColor Yellow
    }
    Write-Host ""
    Write-Host "如需重新安装，请使用 -Force 参数" -ForegroundColor Yellow
    Write-Host "或运行: .\installer.ps1 -Force" -ForegroundColor White
    exit 0
}

# 如果使用 Force 参数，显示重新安装信息
if ($Force) {
    Write-Host "使用 -Force 参数，将强制重新安装 FNM" -ForegroundColor Yellow
    if ($fnmInstalled) {
        try {
            $currentVersion = & fnm --version 2>$null
            Write-Host "当前已安装版本: $currentVersion" -ForegroundColor Cyan
        } catch {
            Write-Host "检测到 FNM 安装但无法获取版本信息" -ForegroundColor Yellow
        }
    }
    Write-Host ""
}

# 确定安装路径
if (-not $InstallPath) {
    $defaultPath = "C:\fnm"
    $InstallPath = Read-Host "请输入 FNM 安装路径 (留空使用默认路径 $defaultPath)"
    if (-not $InstallPath) {
        $InstallPath = $defaultPath
    }
}

# 创建安装目录
Write-Host "创建安装目录..." -ForegroundColor Yellow
try {
    if (!(Test-Path $InstallPath)) {
        New-Item -ItemType Directory -Path $InstallPath -Force | Out-Null
        Write-Host "已创建目录: $InstallPath" -ForegroundColor Green
    }
} catch {
    Write-Host "创建目录失败: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# 检查本地fnm.exe
Write-Host ""
Write-Host "检查fnm.exe文件..." -ForegroundColor Yellow
$localFnmExe = Test-LocalFnmExe $FnmExePath
if (-not $localFnmExe) {
    Write-Host ""
    Write-Host "安装失败: 缺少fnm.exe文件" -ForegroundColor Red
    if ($FnmExePath) {
        Write-Host "请检查指定的文件路径: $FnmExePath" -ForegroundColor Yellow
    } else {
        Write-Host "请下载 fnm.exe 文件并:" -ForegroundColor Yellow
        Write-Host "1. 放置在脚本同目录下，或" -ForegroundColor Yellow
        Write-Host "2. 使用 -FnmExePath 参数指定文件路径" -ForegroundColor Yellow
    }
    Write-Host "下载地址: https://github.com/Schniz/fnm/releases/latest" -ForegroundColor Cyan
    exit 1
}

# 复制fnm.exe文件
Write-Host ""
Write-Host "复制fnm.exe到安装目录..." -ForegroundColor Yellow
Write-Host "安装路径: $InstallPath" -ForegroundColor Cyan
Write-Host ""

try {
    # 如果使用 Force 参数，清理整个安装目录
    if ($Force -and (Test-Path $InstallPath)) {
        Write-Host "清理现有安装目录..." -ForegroundColor Yellow
        # 保留目录结构，但清理内容
        Get-ChildItem -Path $InstallPath -Recurse | Remove-Item -Force -Recurse -ErrorAction SilentlyContinue
        Write-Host "已清理现有安装" -ForegroundColor Green
    }
    
    # 确定目标文件路径
    $targetFnmExe = Join-Path $InstallPath "fnm.exe"
    
    # 复制 fnm.exe 到安装目录
    Copy-Item -Path $localFnmExe -Destination $targetFnmExe -Force
    
    # 验证复制是否成功
    if (Test-Path $targetFnmExe) {
        $targetFileSize = (Get-Item $targetFnmExe).Length
        Write-Host "✓ fnm.exe 复制成功" -ForegroundColor Green
        Write-Host "目标路径: $targetFnmExe" -ForegroundColor Cyan
        Write-Host "文件大小: $([math]::Round($targetFileSize/1MB, 2)) MB" -ForegroundColor Cyan
    } else {
        throw "复制后未找到目标文件"
    }
    
    Write-Host "文件复制完成" -ForegroundColor Green
} catch {
    Write-Host "文件复制失败: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# 配置环境变量
Write-Host ""
Write-Host "配置环境变量..." -ForegroundColor Yellow

# 设置 FNM 相关环境变量
$fnmDir = $InstallPath
$fnmMultishellsDir = Join-Path $fnmDir "multishells"
$fnmNodeDistMirror = if ($UseChineseMirror) { "https://npmmirror.com/mirrors/node" } else { "" }

# 永久设置环境变量
try {
    # FNM_DIR - FNM 安装目录
    [Environment]::SetEnvironmentVariable("FNM_DIR", $fnmDir, "User")
    $env:FNM_DIR = $fnmDir
    Write-Host "已设置 FNM_DIR: $fnmDir" -ForegroundColor Green
    
    # FNM_MULTISHELL_PATH - 多 shell 支持路径
    [Environment]::SetEnvironmentVariable("FNM_MULTISHELL_PATH", $fnmMultishellsDir, "User")
    $env:FNM_MULTISHELL_PATH = $fnmMultishellsDir
    Write-Host "已设置 FNM_MULTISHELL_PATH: $fnmMultishellsDir" -ForegroundColor Green
    
    # FNM_NODE_DIST_MIRROR - Node.js 下载镜像（如果使用国内镜像）
    if ($fnmNodeDistMirror) {
        [Environment]::SetEnvironmentVariable("FNM_NODE_DIST_MIRROR", $fnmNodeDistMirror, "User")
        $env:FNM_NODE_DIST_MIRROR = $fnmNodeDistMirror
        Write-Host "已设置 FNM_NODE_DIST_MIRROR: $fnmNodeDistMirror" -ForegroundColor Green
    }
    
} catch {
    Write-Host "警告: 环境变量设置失败: $($_.Exception.Message)" -ForegroundColor Yellow
}

# 添加到 PATH
Write-Host "添加 FNM 到 PATH..." -ForegroundColor Yellow
try {
    $currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
    
    # 检查是否已经在 PATH 中
    if ($currentPath -notlike "*$fnmDir*") {
        $newPath = "$fnmDir;$currentPath"
        [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        $env:PATH = "$fnmDir;$env:PATH"
        Write-Host "已添加 FNM 到 PATH" -ForegroundColor Green
    } else {
        Write-Host "FNM 已在 PATH 中" -ForegroundColor Cyan
    }
} catch {
    Write-Host "警告: PATH 设置失败: $($_.Exception.Message)" -ForegroundColor Yellow
    Write-Host "请手动将以下路径添加到 PATH:" -ForegroundColor Cyan
    Write-Host "  $fnmDir" -ForegroundColor White
}

# 验证安装
Write-Host ""
Write-Host "验证 FNM 安装..." -ForegroundColor Yellow
try {
    # 刷新当前会话的环境变量
    $env:PATH = [Environment]::GetEnvironmentVariable("PATH", "User") + ";" + [Environment]::GetEnvironmentVariable("PATH", "Machine")
    
    # 测试 FNM 命令
    $fnmExePath = Get-Command fnm -ErrorAction SilentlyContinue
    if ($fnmExePath) {
        $version = & fnm --version 2>$null
        Write-Host "✓ FNM 安装成功!" -ForegroundColor Green
        Write-Host "版本: $version" -ForegroundColor Cyan
        Write-Host "安装路径: $($fnmExePath.Source)" -ForegroundColor Cyan
    } else {
        Write-Host "警告: 无法在 PATH 中找到 fnm 命令" -ForegroundColor Yellow
        Write-Host "请重新启动 PowerShell 后再试" -ForegroundColor Cyan
    }
} catch {
    Write-Host "警告: 无法验证 FNM 安装" -ForegroundColor Yellow
}

# 配置 PowerShell 集成
Write-Host ""
Write-Host "配置 PowerShell 集成..." -ForegroundColor Yellow
try {
    # 检查 PowerShell 配置文件
    $profilePath = $PROFILE
    $profileDir = Split-Path $profilePath -Parent
    
    if (!(Test-Path $profileDir)) {
        New-Item -ItemType Directory -Path $profileDir -Force | Out-Null
    }
    
    # FNM 初始化命令
    $fnmInit = @"

# FNM (Fast Node Manager) 初始化
if (Get-Command fnm -ErrorAction SilentlyContinue) {
    fnm env --use-on-cd | Out-String | Invoke-Expression
}
"@
    
    # 检查是否已经配置
    $profileExists = Test-Path $profilePath
    $fnmConfigured = $false
    
    if ($profileExists) {
        $profileContent = Get-Content $profilePath -Raw -ErrorAction SilentlyContinue
        $fnmConfigured = $profileContent -like "*fnm env*"
    }
    
    if (-not $fnmConfigured) {
        Add-Content -Path $profilePath -Value $fnmInit -Encoding UTF8
        Write-Host "已添加 FNM 初始化到 PowerShell 配置文件" -ForegroundColor Green
        Write-Host "配置文件路径: $profilePath" -ForegroundColor Cyan
    } else {
        Write-Host "FNM 已在 PowerShell 配置文件中" -ForegroundColor Cyan
    }
    
} catch {
    Write-Host "警告: PowerShell 集成配置失败: $($_.Exception.Message)" -ForegroundColor Yellow
    Write-Host "请手动添加以下内容到 PowerShell 配置文件:" -ForegroundColor Cyan
    Write-Host "fnm env --use-on-cd | Out-String | Invoke-Expression" -ForegroundColor White
}

# 安装最新 Node.js (可选)
if ($InstallLatestNode) {
    Write-Host ""
    Write-Host "安装最新 LTS Node.js..." -ForegroundColor Green
    
    try {
        # 刷新环境
        $env:PATH = [Environment]::GetEnvironmentVariable("PATH", "User") + ";" + [Environment]::GetEnvironmentVariable("PATH", "Machine")
        
        # 安装最新 LTS
        Write-Host "正在安装最新 LTS Node.js，请稍候..." -ForegroundColor Yellow
        & fnm install --lts
        & fnm use lts-latest
        & fnm default lts-latest
        
        # 验证 Node.js 安装
        $nodeVersion = & node --version 2>$null
        if ($nodeVersion) {
            Write-Host "✓ Node.js 安装成功: $nodeVersion" -ForegroundColor Green
        }
        
        $npmVersion = & npm --version 2>$null
        if ($npmVersion) {
            Write-Host "✓ NPM 版本: $npmVersion" -ForegroundColor Green
        }
        
    } catch {
        Write-Host "警告: Node.js 安装失败: $($_.Exception.Message)" -ForegroundColor Yellow
        Write-Host "请手动运行: fnm install --lts" -ForegroundColor Cyan
    }
}

# 显示完成信息
Write-Host ""
Write-Host "============================================" -ForegroundColor Cyan
Write-Host "             安装完成!" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "安装信息:" -ForegroundColor Yellow
Write-Host "  安装路径: $fnmDir" -ForegroundColor White
Write-Host "  配置文件: $profilePath" -ForegroundColor White
if ($fnmNodeDistMirror) {
    Write-Host "  下载镜像: $fnmNodeDistMirror" -ForegroundColor White
}
Write-Host ""
Write-Host "常用命令:" -ForegroundColor Yellow
Write-Host "  fnm list-remote          # 列出可用的 Node.js 版本" -ForegroundColor White
Write-Host "  fnm install <版本>       # 安装指定版本的 Node.js" -ForegroundColor White
Write-Host "  fnm install --lts        # 安装最新 LTS 版本" -ForegroundColor White
Write-Host "  fnm use <版本>           # 切换到指定版本" -ForegroundColor White
Write-Host "  fnm list                 # 列出已安装的版本" -ForegroundColor White
Write-Host "  fnm default <版本>       # 设置默认版本" -ForegroundColor White
Write-Host "  fnm current              # 显示当前使用的版本" -ForegroundColor White
Write-Host ""
Write-Host "快速开始:" -ForegroundColor Yellow
Write-Host "  fnm install --lts        # 安装最新 LTS 版本" -ForegroundColor White
Write-Host "  fnm use lts-latest       # 使用最新 LTS 版本" -ForegroundColor White
Write-Host "  fnm default lts-latest   # 设为默认版本" -ForegroundColor White
Write-Host ""
Write-Host "注意事项:" -ForegroundColor Yellow
Write-Host "1. 环境变量已永久设置，重启后仍然有效" -ForegroundColor White
Write-Host "2. 建议重新打开 PowerShell 窗口以确保环境变量生效" -ForegroundColor White
Write-Host "3. 如需卸载，请删除安装目录并清理环境变量" -ForegroundColor White
Write-Host ""

# 询问是否重新启动 PowerShell
$restart = Read-Host "是否重新启动 PowerShell 以应用环境变量? (Y/n)"
if ($restart -ne "n" -and $restart -ne "N") {
    Write-Host "正在重新启动 PowerShell..." -ForegroundColor Green
    Start-Process powershell -ArgumentList "-NoProfile", "-ExecutionPolicy", "RemoteSigned"
    exit 0
} else {
    Write-Host "请手动重新启动 PowerShell 以确保环境变量生效" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "感谢使用 FNM 自动安装脚本!" -ForegroundColor Green
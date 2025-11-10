# fnva - Windows PowerShell 启动脚本

param(
    [Parameter(ValueFromRemainingArguments=$true)]
    [string[]]$Arguments
)

# 检测平台和架构
$os = "win32"
$arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }

# 构建二进制文件路径
$platformDir = "$os-$arch"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$binaryPath = Join-Path $scriptDir ".." "platforms" $platformDir "fnva.exe"

# 如果分层结构不存在，尝试扁平结构
if (-not (Test-Path $binaryPath)) {
    $binaryPath = Join-Path $scriptDir ".." "platforms" "fnva.exe"
}

# 检查二进制文件是否存在
if (-not (Test-Path $binaryPath)) {
    Write-Host "错误: 未找到二进制文件" -ForegroundColor Red
    Write-Host "尝试的路径: " -ForegroundColor Yellow
    Write-Host "  1. $(Join-Path $scriptDir ".." "platforms" $platformDir "fnva.exe")" -ForegroundColor Yellow
    Write-Host "  2. $(Join-Path $scriptDir ".." "platforms" "fnva.exe")" -ForegroundColor Yellow
    Write-Host "请运行 'npm run build' 构建二进制文件" -ForegroundColor Yellow
    exit 1
}

# 执行二进制文件
& $binaryPath $Arguments

if ($LASTEXITCODE) {
    exit $LASTEXITCODE
}
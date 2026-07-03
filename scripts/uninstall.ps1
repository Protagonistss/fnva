# fnva 卸载脚本(Windows PowerShell):删 binary + 清 $PROFILE 的 fnva 块 + 从用户 PATH 移除。
#   irm https://raw.githubusercontent.com/Protagonistss/fnva/main/scripts/uninstall.ps1 | iex
# 默认保留 %USERPROFILE%\.fnva;如需彻底删除,末尾有提示。

$ErrorActionPreference = "Stop"
$InstallDir = if ($env:FNVA_INSTALL_DIR) { $env:FNVA_INSTALL_DIR } else { Join-Path $env:USERPROFILE ".fnva\bin" }

# 1. 清 $PROFILE 的 fnva 块(>>> fnva >>> ... <<< fnva <<<)
if (Test-Path $PROFILE) {
    $content = Get-Content $PROFILE -Raw
    if ($content -match ">>> fnva >>>") {
        $content = $content -replace "(?s)\r?\n?# >>> fnva >>>.*?# <<< fnva <<<", ""
        Set-Content -Path $PROFILE -Value $content -NoNewline
        Write-Host "已从 $PROFILE 清除 fnva 块"
    }
}

# 2. 从用户 PATH 移除 InstallDir
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -like "*$InstallDir*") {
    $newPath = ($userPath -split ";" | Where-Object { $_ -and $_ -ne $InstallDir }) -join ";"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "已从用户 PATH 移除 $InstallDir"
}

# 3. 删 binary
$exe = Join-Path $InstallDir "fnva.exe"
if (Test-Path $exe) {
    Remove-Item $exe -Force
    Write-Host "已删除 $exe"
} else {
    Write-Host "$exe 不存在(可能已删)"
}

Write-Host ""
Write-Host "✓ fnva 已卸载"
Write-Host "  配置目录 $env:USERPROFILE\.fnva 已保留;如需彻底删除:"
Write-Host "    Remove-Item -Recurse -Force $env:USERPROFILE\.fnva"
Write-Host ""
Write-Host "重开 PowerShell 使 PATH 变更生效。"

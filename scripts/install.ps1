# fnva 一键安装脚本(Windows PowerShell)。
#   irm https://raw.githubusercontent.com/Protagonistss/fnva/main/scripts/install.ps1 | iex
#   或: powershell -ExecutionPolicy Bypass -File install.ps1
#
# 行为:从 GitHub Release latest 下载 win32-x64.zip → 解压到
# $env:FNVA_INSTALL_DIR(默认 %USERPROFILE%\.fnva\bin)→ 提示 PATH。

$ErrorActionPreference = "Stop"

$Repo = "Protagonistss/fnva"
$UrlBase = "https://github.com/$Repo/releases/latest/download"
# fnva 目前只发布 win32-x64(无 win32-arm64)
$Platform = "win32-x64"
$InstallDir = if ($env:FNVA_INSTALL_DIR) { $env:FNVA_INSTALL_DIR } else { Join-Path $env:USERPROFILE ".fnva\bin" }

Write-Host "Downloading fnva ($Platform) from GitHub Release..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$Tmp = New-Item -ItemType Directory -Path (Join-Path ([System.IO.Path]::GetTempPath()) ("fnva-" + [guid]::NewGuid()))

try {
    Invoke-WebRequest -UseBasicParsing "${UrlBase}/${Platform}.zip" -OutFile (Join-Path $Tmp.FullName "fnva.zip")
    Expand-Archive -Path (Join-Path $Tmp.FullName "fnva.zip") -DestinationPath $Tmp.FullName -Force
    $Dst = Join-Path $InstallDir "fnva.exe"
    Move-Item -Force (Join-Path $Tmp.FullName "fnva.exe") $Dst

    Write-Host ""
    Write-Host "✓ fnva 已安装到 $Dst"
    Write-Host ""

    # 把 InstallDir 加到用户 PATH(若未在)
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$userPath", "User")
        Write-Host "已把 $InstallDir 加到用户 PATH(重开终端生效)"
    }
    Write-Host ""
    Write-Host "下一步:启用 shell 集成,在 PowerShell 配置(\$PROFILE)里加:"
    Write-Host '  fnva env | Invoke-Expression'
    Write-Host ""
    Write-Host "验证:fnva --version"
}
finally {
    Remove-Item -Recurse -Force $Tmp.FullName -ErrorAction SilentlyContinue
}

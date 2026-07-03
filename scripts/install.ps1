# fnva installer (Windows PowerShell).
#   irm https://raw.githubusercontent.com/Protagonistss/fnva/main/scripts/install.ps1 | iex
#   or: powershell -ExecutionPolicy Bypass -File install.ps1
#
# Downloads the platform binary from GitHub Release latest, extracts to
# $env:FNVA_INSTALL_DIR (default %USERPROFILE%\.fnva\bin), and wires PATH +
# shell integration ($PROFILE).

$ErrorActionPreference = "Stop"

$Repo = "Protagonistss/fnva"
$UrlBase = "https://github.com/$Repo/releases/latest/download"
$Platform = "win32-x64"  # fnva ships win32-x64 only (no win32-arm64 yet)
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
    Write-Host "✓ fnva installed to $Dst"
    Write-Host ""

    # add to user PATH if missing
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$userPath", "User")
        Write-Host "Added $InstallDir to user PATH (reopen terminal to take effect)"
    }

    # wire PowerShell shell integration into $PROFILE (>>> fnva >>> block marker for clean uninstall)
    $profileDir = Split-Path $PROFILE -Parent
    if (-not (Test-Path $profileDir)) { New-Item -ItemType Directory -Force -Path $profileDir | Out-Null }
    $needAdd = $true
    if (Test-Path $PROFILE) {
        if ((Get-Content $PROFILE -Raw) -match ">>> fnva >>>") { $needAdd = $false }
    }
    if ($needAdd) {
        Add-Content $PROFILE "`n# >>> fnva >>>`nfnva env | Invoke-Expression`n# <<< fnva <<<"
        Write-Host "Added shell integration to $PROFILE"
    }

    Write-Host ""
    Write-Host "Reopen PowerShell and fnva is ready — verify: fnva --version"
    Write-Host "Uninstall: irm https://raw.githubusercontent.com/$Repo/main/scripts/uninstall.ps1 | iex"
}
finally {
    Remove-Item -Recurse -Force $Tmp.FullName -ErrorAction SilentlyContinue
}

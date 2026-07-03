# fnva uninstaller (Windows PowerShell):
#   remove binary + strip fnva block from $PROFILE + remove from user PATH.
#   irm https://raw.githubusercontent.com/Protagonistss/fnva/main/scripts/uninstall.ps1 | iex
#
# Keeps %USERPROFILE%\.fnva by default; see the note at the end to wipe it.

$ErrorActionPreference = "Stop"
$InstallDir = if ($env:FNVA_INSTALL_DIR) { $env:FNVA_INSTALL_DIR } else { Join-Path $env:USERPROFILE ".fnva\bin" }

# 1. strip fnva block from $PROFILE
if (Test-Path $PROFILE) {
    $content = Get-Content $PROFILE -Raw
    if ($content -match ">>> fnva >>>") {
        $content = $content -replace "(?s)\r?\n?# >>> fnva >>>.*?# <<< fnva <<<", ""
        Set-Content -Path $PROFILE -Value $content -NoNewline
        Write-Host "Removed fnva block from $PROFILE"
    }
}

# 2. remove InstallDir from user PATH
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -like "*$InstallDir*") {
    $newPath = ($userPath -split ";" | Where-Object { $_ -and $_ -ne $InstallDir }) -join ";"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "Removed $InstallDir from user PATH"
}

# 3. remove binary
$exe = Join-Path $InstallDir "fnva.exe"
if (Test-Path $exe) {
    Remove-Item $exe -Force
    Write-Host "Removed $exe"
} else {
    Write-Host "$exe not found (already removed?)"
}

Write-Host ""
Write-Host "✓ fnva uninstalled"
Write-Host "  Config dir $env:USERPROFILE\.fnva kept; to wipe it completely:"
Write-Host "    Remove-Item -Recurse -Force $env:USERPROFILE\.fnva"
Write-Host ""
Write-Host "Reopen PowerShell so PATH changes take effect."

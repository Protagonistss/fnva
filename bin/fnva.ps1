#!/usr/bin/env pwsh
# fnva PowerShell wrapper - calls native binary directly
# PowerShell prefers .ps1 over .cmd, avoiding Object[] output splitting

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
$platformDir = "win32-$arch"

# Try native binary in npm package
$binaryPath = Join-Path $scriptDir "..\platforms\$platformDir\fnva.exe"
if (Test-Path $binaryPath) {
    & $binaryPath @args
    exit $LASTEXITCODE
}

# Fallback: local dev build
$devPath = Join-Path $scriptDir "..\target\release\fnva.exe"
if (Test-Path $devPath) {
    & $devPath @args
    exit $LASTEXITCODE
}

# Last resort: node wrapper
$nodeScript = Join-Path $scriptDir "fnva.js"
if (Test-Path $nodeScript) {
    & node $nodeScript @args
    exit $LASTEXITCODE
}

Write-Error "fnva: native binary not found. Reinstall with: npm install -g fnva --force"
exit 1

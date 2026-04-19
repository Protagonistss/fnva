#!/usr/bin/env pwsh
# fnva PowerShell wrapper - calls native binary directly
# PowerShell prefers .ps1 over .cmd, avoiding Object[] output splitting

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "arm64" } else { "x64" }
$platformDir = "win32-$arch"

# Search paths for native binary (in priority order)
$searchPaths = @(
    # npm global install: fnva.ps1 is in <prefix>/, binary is in <prefix>/node_modules/fnva/platforms/
    (Join-Path $scriptDir "node_modules\fnva\platforms\$platformDir\fnva.exe"),
    # npm global install (flat): binary next to fnva.ps1
    (Join-Path $scriptDir "fnva.exe"),
    # Source tree: fnva.ps1 is in bin/, binary is in platforms/
    (Join-Path $scriptDir "..\platforms\$platformDir\fnva.exe"),
    # Local dev build
    (Join-Path $scriptDir "..\target\release\fnva.exe")
)

foreach ($binaryPath in $searchPaths) {
    if (Test-Path $binaryPath) {
        & $binaryPath @args
        exit $LASTEXITCODE
    }
}

# Last resort: node wrapper
$nodeScript = Join-Path $scriptDir "node_modules\fnva\bin\fnva.js"
if (-not (Test-Path $nodeScript)) {
    $nodeScript = Join-Path $scriptDir "fnva.js"
}
if (Test-Path $nodeScript) {
    & node $nodeScript @args
    exit $LASTEXITCODE
}

Write-Error "fnva: native binary not found. Reinstall with: npm install -g fnva --force"
exit 1

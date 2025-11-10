#!/usr/bin/env node

const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');

function resolvePlatform() {
  switch (process.platform) {
    case 'win32':
    case 'darwin':
    case 'linux':
      return process.platform;
    default:
      throw new Error(`Unsupported platform: ${process.platform}`);
  }
}

function resolveArch() {
  const arch = process.arch;
  if (arch === 'x64') {
    return 'x64';
  }
  if (arch === 'arm64') {
    return 'arm64';
  }
  // Fallback to x64 for unknown architectures to keep previous behaviour.
  return 'x64';
}

function buildBinaryPath() {
  const platform = resolvePlatform();
  const arch = resolveArch();
  const scriptDir = __dirname;
  const projectRoot = path.resolve(scriptDir, '..');
  const platformDir = `${platform}-${arch}`;
  const binaryName = platform === 'win32' ? 'fnva.exe' : 'fnva';
  return path.join(projectRoot, 'platforms', platformDir, binaryName);
}

function run() {
  const binaryPath = buildBinaryPath();

  if (!fs.existsSync(binaryPath)) {
    console.error(`Error: binary not found: ${binaryPath}`);
    console.error('');
    console.error("Please build the CLI binaries first, e.g. run 'npm run build' or 'npm run build:all'.");
    process.exit(1);
  }

  const result = spawnSync(binaryPath, process.argv.slice(2), {
    stdio: 'inherit',
  });

  if (result.error) {
    console.error(`Failed to execute fnva: ${result.error.message}`);
    process.exit(result.status ?? 1);
  }

  process.exit(result.status ?? 0);
}

run();

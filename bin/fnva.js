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

function platformBinaryPath(platformOverride) {
  const platform = platformOverride || resolvePlatform();
  const arch = resolveArch();
  const scriptDir = __dirname;
  const projectRoot = path.resolve(scriptDir, '..');
  const platformDir = `${platform}-${arch}`;
  const binaryName = platform === 'win32' ? 'fnva.exe' : 'fnva';
  return path.join(projectRoot, 'platforms', platformDir, binaryName);
}

function buildBinaryPath() {
  const platform = resolvePlatform();
  const binaryCandidates = [];

  // 1. Prebuilt binary shipped with the npm package
  binaryCandidates.push(platformBinaryPath(platform));

  // Flat legacy structure: platforms/fnva(.exe)
  const scriptDir = __dirname;
  const projectRoot = path.resolve(scriptDir, '..');
  const flatBinaryName = platform === 'win32' ? 'fnva.exe' : 'fnva';
  binaryCandidates.push(path.join(projectRoot, 'platforms', flatBinaryName));

  // 2. User-provided override via environment variable
  if (process.env.FNVA_NATIVE_PATH) {
    binaryCandidates.push(process.env.FNVA_NATIVE_PATH);
  }

  // 3. Local cargo build outputs (helpful for development installs)
  const targetDir = path.resolve(__dirname, '..', 'target');
  if (platform === 'win32') {
    binaryCandidates.push(path.join(targetDir, 'release', 'fnva.exe'));
    binaryCandidates.push(path.join(targetDir, 'debug', 'fnva.exe'));
  } else {
    binaryCandidates.push(path.join(targetDir, 'release', 'fnva'));
    binaryCandidates.push(path.join(targetDir, 'debug', 'fnva'));
  }

  for (const candidate of binaryCandidates) {
    if (candidate && fs.existsSync(candidate)) {
      return candidate;
    }
  }

  return null;
}

function run() {
  const binaryPath = buildBinaryPath();

  if (!binaryPath) {
    console.error('Error: fnva native binary not found.');
    console.error('');
    console.error("Please either:");
    console.error("  1) Run 'npm run build' (or 'npm run build:all') to produce platform binaries,");
    console.error("  2) Install a release package that includes the platforms directory, or");
    console.error("  3) Set FNVA_NATIVE_PATH to the full path of an existing fnva executable.");
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

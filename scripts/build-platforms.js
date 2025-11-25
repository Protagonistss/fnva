#!/usr/bin/env node

/**
 * Cross-platform build helper to compile fnva binaries for common targets.
 * Uses the current host toolchain; make sure required targets are installed via `rustup target add ...`.
 */

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..');

const TARGETS = [
  { target: 'aarch64-apple-darwin', platform: 'darwin-arm64', bin: 'fnva' },
  { target: 'x86_64-apple-darwin', platform: 'darwin-x64', bin: 'fnva' },
  { target: 'x86_64-unknown-linux-gnu', platform: 'linux-x64', bin: 'fnva' },
  { target: 'x86_64-pc-windows-msvc', platform: 'win32-x64', bin: 'fnva.exe' },
];

function run(cmd, args, options = {}) {
  const result = spawnSync(cmd, args, { stdio: 'inherit', ...options });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`${cmd} ${args.join(' ')} exited with code ${result.status}`);
  }
}

function commandExists(cmd) {
  const probe = spawnSync(cmd, ['--version'], { stdio: 'ignore' });
  return probe.error == null && probe.status === 0;
}

function targetInstalled(target) {
  const res = spawnSync('rustup', ['target', 'list', '--installed'], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  });
  if (res.error || typeof res.stdout !== 'string') {
    return false;
  }
  return res.stdout.split(/\r?\n/).some((line) => line.trim() === target);
}

function buildTarget(entry) {
  const { target, platform, bin } = entry;
  console.log(`==> Building ${target} -> platforms/${platform}/${bin}`);

  if (!commandExists('cargo')) {
    console.error('!! cargo not found; install Rust toolchain first');
    process.exitCode = 1;
    return;
  }

  if (!targetInstalled(target)) {
    console.warn(`!! target ${target} not installed; run: rustup target add ${target}`);
    return;
  }

  run('cargo', ['build', '--release', '--target', target], { cwd: ROOT });

  const source = path.join(ROOT, 'target', target, 'release', bin);
  const destDir = path.join(ROOT, 'platforms', platform);
  const dest = path.join(destDir, bin);

  if (!fs.existsSync(source)) {
    console.warn(`!! Missing build output: ${source}`);
    return;
  }

  fs.mkdirSync(destDir, { recursive: true });
  fs.copyFileSync(source, dest);
}

function main() {
  TARGETS.forEach(buildTarget);
  console.log('==> Done. Binaries are in platforms/<os>-<arch>/');
}

main();

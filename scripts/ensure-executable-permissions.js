#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

function log(msg) {
  console.log(msg);
}

function ensureExecutable(filePath, label) {
  try {
    const stats = fs.statSync(filePath);
    const hasExec = (stats.mode & 0o111) !== 0;
    log(`Checking ${label}: ${filePath}`);
    log(`   Current permissions: ${(stats.mode & 0o777).toString(8)}`);

    if (!hasExec) {
      fs.chmodSync(filePath, 0o755);
      const newStats = fs.statSync(filePath);
      log(`   Updated permissions: ${(newStats.mode & 0o777).toString(8)}`);
    } else {
      log('   Executable bit already set');
    }

    const res = spawnSync(filePath, ['--version'], { encoding: 'utf8', timeout: 3000, stdio: 'pipe' });
    if (res.error && res.error.code === 'EACCES') {
      log('WARNING: still not executable; please chmod +x manually');
    }
  } catch (err) {
    log(`WARNING: could not ensure permissions for ${label}: ${err.message}`);
  }
}

function ensureExecutablePermissions() {
  const scriptDir = __dirname;
  const projectRoot = path.resolve(scriptDir, '..');
  const platformsDir = path.join(projectRoot, 'platforms');

  log('Ensuring fnva binary permissions...');

  if (!fs.existsSync(platformsDir)) {
    log('Info: no platforms directory found; skipping (dev install)');
    return;
  }

  const platform = process.platform;
  const arch = process.arch === 'arm64' ? 'arm64' : 'x64';
  const platformDir = `${platform}-${arch}`;
  const binaryName = platform === 'win32' ? 'fnva.exe' : 'fnva';
  const archBinaryPath = path.join(platformsDir, platformDir, binaryName);
  const flatBinaryPath = path.join(platformsDir, binaryName);

  if (platform === 'win32') {
    log('Info: Windows detected; chmod not required; skipping permission changes');
    return;
  }

  if (fs.existsSync(archBinaryPath)) {
    ensureExecutable(archBinaryPath, platformDir);
  } else if (fs.existsSync(flatBinaryPath)) {
    log('Info: falling back to legacy platforms/fnva layout');
    ensureExecutable(flatBinaryPath, 'platforms/fnva');
  } else {
    log(`Warning: binary not found: ${archBinaryPath}`);
    log(`         Also checked: ${flatBinaryPath}`);
  }

  if (process.env.npm_config_global === 'true') {
    try {
      const which = spawnSync('which', ['fnva'], { encoding: 'utf8' });
      const globalPath = which.stdout?.trim();
      if (globalPath && fs.existsSync(globalPath)) {
        ensureExecutable(globalPath, 'global fnva');
      }
    } catch {
      // ignore
    }
  }
}

ensureExecutablePermissions();
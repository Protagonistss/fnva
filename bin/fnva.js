#!/usr/bin/env node

const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const EncodingUtils = require('../lib/encoding-utils');

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
  // 如果设置了 FNVA_SKIP_NATIVE，跳过原生二进制查找
  if (process.env.FNVA_SKIP_NATIVE === '1') {
    return null;
  }

  const platform = resolvePlatform();
  const binaryCandidates = [];

  // 1. Prebuilt binary shipped with the npm package
  const npmBinaryPath = platformBinaryPath(platform);
  binaryCandidates.push(npmBinaryPath);

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

  // Debug: Show all candidates and their existence
  if (process.env.FNVA_DEBUG === '1') {
    console.log('[DEBUG] Looking for fnva binary...');
    console.log('[DEBUG] Platform:', platform, 'Arch:', resolveArch());
    console.log('[DEBUG] Binary candidates:');
    binaryCandidates.forEach((candidate, index) => {
      const exists = candidate && fs.existsSync(candidate);
      console.log(`  ${index + 1}. ${candidate} - ${exists ? 'EXISTS' : 'MISSING'}`);
    });
  }

  for (const candidate of binaryCandidates) {
    if (candidate && fs.existsSync(candidate)) {
      if (process.env.FNVA_DEBUG === '1') {
        console.log(`[DEBUG] Found binary at: ${candidate}`);
      }
      return candidate;
    }
  }

  if (process.env.FNVA_DEBUG === '1') {
    console.log('[DEBUG] No binary found, falling back to Node.js mode');
    console.log('[DEBUG] Expected npm package binary path:', npmBinaryPath);
    console.log('[DEBUG] npmBinaryPath exists:', fs.existsSync(npmBinaryPath));
  }

  return null;
}

function isEnvironmentSwitchCommand(args) {
  return args.length >= 3 &&
         (args[0] === 'java' || args[0] === 'cc') &&
         args[1] === 'use';
}

function getShellArg(args) {
  const idx = args.indexOf('--shell');
  if (idx !== -1 && idx + 1 < args.length) {
    return args[idx + 1];
  }
  return null;
}

function detectShell() {
  if (process.platform === 'win32') {
    return 'powershell';
  }
  return 'bash';
}








async function checkFirstRun() {
  const os = require('os');
  const fs = require('fs');
  const path = require('path');
  const setupMarker = path.join(os.homedir(), '.fnva', '.shell_setup_done');

  if (fs.existsSync(setupMarker) || process.env.FNVA_SKIP_SHELL_SETUP === '1') {
    return;
  }

  const fnvaDir = path.join(os.homedir(), '.fnva');
  if (!fs.existsSync(fnvaDir)) {
    fs.mkdirSync(fnvaDir, { recursive: true });
  }

  const installerPath = path.join(__dirname, '..', 'scripts', 'install-shell-integration.js');
  if (!fs.existsSync(installerPath)) {
    return; // Script not shipped
  }

  const installer = require(installerPath);
  const shell = installer.detectShell();
  const configPath = installer.getShellConfigPath(shell);

  if (!configPath) {
    fs.writeFileSync(setupMarker, 'skipped');
    return;
  }

  if (installer.isInstalled(configPath)) {
    fs.writeFileSync(setupMarker, 'installed');
    return;
  }

  console.log('🚀 Welcome to fnva! We detected that this is your first run.');
  console.log(`To allow fnva to automatically manage shell environment variables, we need to append a line of code to your shell configuration file (${configPath}).`);

  return new Promise((resolve) => {
    const readline = require('readline');
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout,
    });

    rl.question('? Do you allow automatic configuration of terminal integration? (Y/n) ', (answer) => {
      const normalized = answer.trim().toLowerCase();
      if (normalized === '' || normalized === 'y' || normalized === 'yes') {
        if (typeof installer.installPowershellWrapper === 'function') {
          installer.installPowershellWrapper();
        }
        const success = installer.installShellIntegration(true);
        if (success) {
          console.log('\n✅ Shell integration has been successfully configured!');
          console.log('💡 To apply the changes immediately, please run:\n');
          console.log(`   source ${configPath}\n`);
          console.log('   (Or simply close and reopen your terminal)');
        }
      } else {
        console.log('\n⏭️  Skipped automatic configuration.');
        console.log("💡 You can manually configure it later. See the README or run 'fnva env --help' for instructions.");
      }
      fs.writeFileSync(setupMarker, answer);
      rl.close();
      console.log('');
      resolve();
    });
  });
}

async function run() {
  await checkFirstRun();

  // 设置Windows控制台编码
  EncodingUtils.setWindowsConsoleEncoding();

  // 强制显示调试信息
  const showDebug = process.env.FNVA_DEBUG === '1' || process.argv.includes('--debug');

  if (showDebug) {
    console.log('=== FNVA DEBUG INFORMATION ===');
    console.log('Node.js version:', process.version);
    console.log('Platform:', process.platform);
    console.log('Architecture:', process.arch);
    console.log('Node binary:', process.execPath);
    console.log('Script directory:', __dirname);
    console.log('Working directory:', process.cwd());
    console.log('Environment variables:');
    console.log('  FNVA_DEBUG:', process.env.FNVA_DEBUG);
    console.log('  FNVA_SKIP_NATIVE:', process.env.FNVA_SKIP_NATIVE);
    console.log('Command line args:', process.argv);
    console.log('');
  }

  const binaryPath = buildBinaryPath();

  if (binaryPath && process.platform !== 'win32') {
    try {
      const fs = require('fs');
      const stats = fs.statSync(binaryPath);
      const hasExec = (stats.mode & 0o111) !== 0;
      if (!hasExec) {
        fs.chmodSync(binaryPath, 0o755);
      }
    } catch (e) {
      // ignore errors, let spawnSync handle it
    }
  }

  if (showDebug) {
    console.log('=== BINARY SEARCH RESULTS ===');
    console.log('Binary path found:', binaryPath);

    // 手动检查所有可能的路径
    const fs = require('fs');
    const path = require('path');

    const scriptDir = __dirname;
    const projectRoot = path.resolve(scriptDir, '..');
    const platform = process.platform;
    const arch = process.arch;
    const platformDir = `${platform}-${arch}`;
    const binaryName = platform === 'win32' ? 'fnva.exe' : 'fnva';
    const expectedPath = path.join(projectRoot, 'platforms', platformDir, binaryName);

    console.log('Expected binary path:', expectedPath);
    console.log('Expected path exists:', fs.existsSync(expectedPath));

    // 检查platforms目录结构
    console.log('');
    console.log('=== PLATFORMS DIRECTORY ===');
    const platformsDir = path.join(projectRoot, 'platforms');
    if (fs.existsSync(platformsDir)) {
      const platforms = fs.readdirSync(platformsDir, { withFileTypes: true });
      platforms.forEach(item => {
        if (item.isDirectory()) {
          const platformPath = path.join(platformsDir, item.name);
          const files = fs.readdirSync(platformPath);
          console.log(`platforms/${item.name}/:`, files);
        }
      });
    } else {
      console.log('platforms directory does not exist');
    }

    console.log('=== END DEBUG ===');
    console.log('');
  }

  if (!binaryPath) {
    console.error('❌ Error: fnva native binary not found.');
    console.error('');

    if (showDebug) {
      console.error('🔍 Debug information is shown above');
      console.error('');
    }

    console.error("💡 Solutions:");
    console.error("  1) Reinstall npm package: npm install -g fnva --force");
    console.error("  2) Download binary from GitHub Release");
    console.error("  3) Set FNVA_DEBUG=1 to show debug information");
    process.exit(1);
  }

  let args = process.argv.slice(2);

  // 如果设置了 FNVA_AUTO_EXECUTE，则为环境切换命令启用自动执行
  if (process.env.FNVA_AUTO_EXECUTE === '1' && isEnvironmentSwitchCommand(args)) {
    args = args.concat('--auto');
  }

  // 对于 env 输出命令，我们需要特殊处理捕获输出以防拆行
  const isEnvOutputCommand = args[0] === 'env' && args[1] === 'env';
  if (isEnvOutputCommand) {
    const { spawnSync } = require('child_process');
    const result = spawnSync(binaryPath, args, {
      encoding: 'utf8',
      shell: false,
    });

    if (result.error) {
      console.error(`[ERROR] Failed to execute fnva: ${result.error.message}`);
      process.exit(result.status ?? 1);
    }

    if (result.stdout) {
      process.stdout.write(result.stdout);
    }
    if (result.stderr) {
      process.stderr.write(result.stderr);
    }

    process.exit(result.status ?? 0);
  }

  // 统一所有平台：直接用 stdio: 'inherit' 透传，依靠 wrapper 处理脚本 sourcing
  const { spawnSync } = require('child_process');
  const result = spawnSync(binaryPath, args, {
    stdio: 'inherit',
  });

  if (result.error) {
    if (result.error.code === 'EACCES' && process.platform !== 'win32') {
      console.error(`[ERROR] Permission denied. The fnva binary is not executable.`);
      console.error(`[INFO] To fix this, run: sudo chmod +x "${binaryPath}"`);
      console.error(`[INFO] Or reinstall: npm install -g fnva --force`);
    } else {
      console.error(`[ERROR] Failed to execute fnva: ${result.error.message}`);
    }
    process.exit(result.status ?? 1);
  }

  process.exit(result.status ?? 0);
}

run();

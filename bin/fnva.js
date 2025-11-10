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

function isEnvironmentSwitchCommand(args) {
  return args.length >= 3 &&
         (args[0] === 'java' || args[0] === 'llm' || args[0] === 'cc') &&
         args[1] === 'use';
}

function parseEnvironmentScript(scriptContent) {
  if (!scriptContent || scriptContent.trim() === '') {
    return {};
  }

  // 将数组输出转换为字符串
  if (Array.isArray(scriptContent)) {
    scriptContent = scriptContent.join('\n');
  }

  const envVars = {};
  const lines = scriptContent.split('\n');

  for (const line of lines) {
    const trimmedLine = line.trim();

    // 解析 PowerShell 环境变量设置
    if (trimmedLine.startsWith('$env:')) {
      const match = trimmedLine.match(/\$env:(\w+)\s*=\s*"([^"]*)"/);
      if (match) {
        envVars[match[1]] = match[2];
      }
    }

    // 解析 bash/zsh 环境变量设置
    if (trimmedLine.startsWith('export ')) {
      const match = trimmedLine.match(/export\s+(\w+)\s*=\s*"([^"]*)"/);
      if (match) {
        envVars[match[1]] = match[2];
      }
    }

    // 解析不带引号的环境变量设置
    const unquotedMatch = trimmedLine.match(/\$env:(\w+)\s*=\s*([^;]+)/);
    if (unquotedMatch) {
      envVars[unquotedMatch[1]] = unquotedMatch[2].trim();
    }
  }

  return envVars;
}

function applyEnvironmentVariables(envVars) {
  for (const [key, value] of Object.entries(envVars)) {
    process.env[key] = value;
  }
}

function displaySuccessMessage(envType, envName, envVars) {
  console.log(`✅ Switched to ${envType} environment: ${envName}`);

  if (envVars.JAVA_HOME) {
    console.log(`📁 JAVA_HOME: ${envVars.JAVA_HOME}`);
  }

  if (envVars.ANTHROPIC_AUTH_TOKEN) {
    console.log(`🔑 ANTHROPIC_AUTH_TOKEN: [已设置]`);
  }

  if (envVars.OPENAI_API_KEY) {
    console.log(`🔑 OPENAI_API_KEY: [已设置]`);
  }
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

  const args = process.argv.slice(2);
  const isSwitchCommand = isEnvironmentSwitchCommand(args);

  if (isSwitchCommand) {
    // 对于环境切换命令，检查是否在管道模式
    const isPipedOutput = !process.stdout.isTTY;

    const { spawnSync } = require('child_process');
    const result = spawnSync(binaryPath, args, {
      encoding: 'utf8',
      shell: true
    });

    if (result.error) {
      console.error(`Failed to execute fnva: ${result.error.message}`);
      process.exit(result.status ?? 1);
    }

    if (result.status !== 0) {
      process.exit(result.status);
    }

    // 获取环境切换脚本
    const stdout = result.stdout || '';
    if (stdout.includes('JAVA_HOME') || stdout.includes('ANTHROPIC_') || stdout.includes('OPENAI_')) {
      // 将数组输出转换为字符串
      const script = Array.isArray(stdout) ? stdout.join('\n') : stdout;

      if (isPipedOutput) {
        // 管道模式：只输出纯净的脚本
        console.log(script);
      } else {
        // 交互模式：显示详细信息和选项
        const envType = args[0];
        const envName = args[2];

        console.log(`✅ Switched to ${envType} environment: ${envName}`);
        console.log('');
        console.log('📝 Environment script ready. To apply it:');
        console.log('');
        console.log('Method 1 (Recommended): Copy and paste this into PowerShell:');
        console.log('----------------------------------------');
        console.log(script);
        console.log('----------------------------------------');
        console.log('');
        console.log('Method 2 (One-line):');
        console.log('node bin/fnva.js java use ' + envName + ' | powershell -Command -');
        console.log('');
        console.log('Method 3 (Save and execute):');
        console.log('node bin/fnva.js java use ' + envName + ' > temp.ps1 && powershell -ExecutionPolicy Bypass -File temp.ps1 && del temp.ps1');
        console.log('');
        console.log('💡 After applying, test with: java --version');
      }
    } else {
      // 如果不是环境脚本，直接输出
      console.log(stdout);
    }

    // 如果有 stderr 输出，也显示出来
    if (result.stderr) {
      console.error(result.stderr);
    }

    process.exit(0);
  } else {
    // 对于其他命令，使用原有的 stdio: 'inherit' 方式
    const result = spawnSync(binaryPath, args, {
      stdio: 'inherit',
    });

    if (result.error) {
      console.error(`Failed to execute fnva: ${result.error.message}`);
      process.exit(result.status ?? 1);
    }

    process.exit(result.status ?? 0);
  }
}

run();

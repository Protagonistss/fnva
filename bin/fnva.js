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
         (args[0] === 'java' || args[0] === 'llm' || args[0] === 'cc') &&
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

function hasSessionFlag(args) {
  return args.includes('--session');
}

function hasApplyFlag(args) {
  return args.includes('--apply');
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
    if (trimmedLine.includes('$env:')) {
      // 匹配 $env:VARNAME = "value" 格式
      let match = trimmedLine.match(/\$env:(\w+)\s*=\s*"([^"]*)"/);
      if (match) {
        envVars[match[1]] = match[2];
        continue;
      }

      // 匹配 $env:VARNAME = 'value' 格式
      match = trimmedLine.match(/\$env:(\w+)\s*=\s*'([^']*)'/);
      if (match) {
        envVars[match[1]] = match[2];
        continue;
      }

      // 匹配 $env:VARNAME = value 格式（不带引号）
      match = trimmedLine.match(/\$env:(\w+)\s*=\s*([^;]+)/);
      if (match) {
        envVars[match[1]] = match[2].trim().replace(/['"]/g, '');
      }
    }

    // 解析 bash/zsh 环境变量设置
    if (trimmedLine.startsWith('export ')) {
      const match = trimmedLine.match(/export\s+(\w+)\s*=\s*"([^"]*)"/);
      if (match) {
        envVars[match[1]] = match[2];
      }
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
  console.log(`[OK] Switched to ${envType} environment: ${envName}`);

  if (envVars.JAVA_HOME) {
    console.log(`[DIR] JAVA_HOME: ${envVars.JAVA_HOME}`);
  }

  if (envVars.ANTHROPIC_AUTH_TOKEN) {
    console.log(`[KEY] ANTHROPIC_AUTH_TOKEN: [已设置]`);
  }

  if (envVars.OPENAI_API_KEY) {
    console.log(`[KEY] OPENAI_API_KEY: [已设置]`);
  }
}

function generateSimpleScript(envVars, envType, envName) {
  const lines = [];

  if (process.platform === 'win32') {
    // Windows PowerShell - 使用编码工具设置
    lines.push(EncodingUtils.generatePowerShellEncodingSetup());
    lines.push(`Write-Host "Switched to ${envType} environment: ${envName}" -ForegroundColor Green`);

    if (envVars.JAVA_HOME) {
      lines.push(`$env:JAVA_HOME = "${envVars.JAVA_HOME}"`);
      // 对于 PATH，我们需要智能处理：移除旧的 Java 路径，添加新的
      lines.push(`# Remove existing Java paths from PATH`);
      lines.push(`$pathParts = $env:PATH -split ';'`);
      lines.push(`$cleanPath = @()`);
      lines.push(`foreach ($part in $pathParts) {`);
      lines.push(`    if ($part -notmatch 'java' -and $part -notmatch 'jdk') {`);
      lines.push(`        $cleanPath += $part`);
      lines.push(`    }`);
      lines.push(`}`);
      lines.push(`$env:PATH = "${envVars.JAVA_HOME}\\bin;" + ($cleanPath -join ';')`);
      lines.push(`Write-Host "JAVA_HOME: $env:JAVA_HOME" -ForegroundColor Yellow`);
    }

    if (envVars.ANTHROPIC_AUTH_TOKEN) {
      lines.push(`$env:ANTHROPIC_AUTH_TOKEN = "${envVars.ANTHROPIC_AUTH_TOKEN}"`);
      lines.push(`Write-Host "ANTHROPIC_AUTH_TOKEN: [已设置]" -ForegroundColor Yellow`);
    }

    if (envVars.ANTHROPIC_BASE_URL) {
      lines.push(`$env:ANTHROPIC_BASE_URL = "${envVars.ANTHROPIC_BASE_URL}"`);
      lines.push(`Write-Host "ANTHROPIC_BASE_URL: ${envVars.ANTHROPIC_BASE_URL}" -ForegroundColor Yellow`);
    }

    if (envVars.ANTHROPIC_DEFAULT_SONNET_MODEL) {
      lines.push(`$env:ANTHROPIC_DEFAULT_SONNET_MODEL = "${envVars.ANTHROPIC_DEFAULT_SONNET_MODEL}"`);
      lines.push(`Write-Host "ANTHROPIC_DEFAULT_SONNET_MODEL: ${envVars.ANTHROPIC_DEFAULT_SONNET_MODEL}" -ForegroundColor Yellow`);
    }

    if (envVars.OPENAI_API_KEY) {
      lines.push(`$env:OPENAI_API_KEY = "${envVars.OPENAI_API_KEY}"`);
      lines.push(`Write-Host "OPENAI_API_KEY: [已设置]" -ForegroundColor Yellow`);
    }
  } else {
    // Unix-like systems
    lines.push(`echo "Switched to ${envType} environment: ${envName}"`);

    if (envVars.JAVA_HOME) {
      lines.push(`export JAVA_HOME="${envVars.JAVA_HOME}"`);
      // 对于 PATH，我们也需要智能处理
      lines.push(`# Remove existing Java paths from PATH`);
      lines.push(`echo $PATH | tr ':' '\\n' | grep -v java | grep -v jdk | tr '\\n' ':' | sed 's/:$//' > /tmp/clean_path`);
      lines.push(`export PATH="${envVars.JAVA_HOME}/bin:$(cat /tmp/clean_path)"`);
      lines.push(`rm -f /tmp/clean_path`);
      lines.push(`echo "JAVA_HOME: $JAVA_HOME"`);
    }

    if (envVars.ANTHROPIC_AUTH_TOKEN) {
      lines.push(`export ANTHROPIC_AUTH_TOKEN="${envVars.ANTHROPIC_AUTH_TOKEN}"`);
      lines.push(`echo "ANTHROPIC_AUTH_TOKEN: [已设置]"`);
    }

    if (envVars.ANTHROPIC_BASE_URL) {
      lines.push(`export ANTHROPIC_BASE_URL="${envVars.ANTHROPIC_BASE_URL}"`);
      lines.push(`echo "ANTHROPIC_BASE_URL: ${envVars.ANTHROPIC_BASE_URL}"`);
    }

    if (envVars.ANTHROPIC_DEFAULT_SONNET_MODEL) {
      lines.push(`export ANTHROPIC_DEFAULT_SONNET_MODEL="${envVars.ANTHROPIC_DEFAULT_SONNET_MODEL}"`);
      lines.push(`echo "ANTHROPIC_DEFAULT_SONNET_MODEL: ${envVars.ANTHROPIC_DEFAULT_SONNET_MODEL}"`);
    }

    if (envVars.OPENAI_API_KEY) {
      lines.push(`export OPENAI_API_KEY="${envVars.OPENAI_API_KEY}"`);
      lines.push(`echo "OPENAI_API_KEY: [已设置]"`);
    }
  }

  return lines.join('\n');
}

function handleNodeOnlyMode(args) {
  const fs = require('fs');
  const path = require('path');
  const os = require('os');

  // 只支持基本帮助信息
  if (args.length === 0 || args[0] === '--help' || args[0] === '-h') {
    console.log('fnva - 环境管理工具 (Node.js 降级模式)');
    console.log('');
    console.log('⚠️  当前运行在 Node.js 降级模式，功能有限');
    console.log('');
    console.log('解决方法:');
    console.log('1. 确保 npm 包包含平台二进制文件');
    console.log('2. 重新安装: npm install -g fnva --force');
    console.log('3. 或者直接下载原生二进制文件');
    console.log('');
    console.log('临时可用功能:');
    console.log('  java list     - 列出 Java 环境');
    console.log('  java use <n>  - 切换 Java 环境');
    return;
  }

  if (args[0] === 'java') {
    const homeDir = os.homedir();
    const fnvaDir = path.join(homeDir, '.fnva', 'java-packages');

    if (args[1] === 'list') {
      if (!fs.existsSync(fnvaDir)) {
        console.log('No Java environments found');
        return;
      }

      const versions = fs.readdirSync(fnvaDir, { withFileTypes: true })
        .filter(dirent => dirent.isDirectory())
        .map(dirent => dirent.name)
        .sort();

      if (versions.length === 0) {
        console.log('No Java environments found');
      } else {
        console.log('Available java environments:');
        versions.forEach(version => {
          const versionDir = path.join(fnvaDir, version);
          const jdkSubdirs = fs.readdirSync(versionDir, { withFileTypes: true })
            .filter(dirent => dirent.isDirectory())
            .map(dirent => dirent.name);

          if (jdkSubdirs.length > 0) {
            const jdkSubdir = jdkSubdirs[0];
            const fullJdkPath = path.join(versionDir, jdkSubdir);

            if (fs.existsSync(path.join(fullJdkPath, 'release'))) {
              try {
                const releaseContent = fs.readFileSync(path.join(fullJdkPath, 'release'), 'utf8');
                const versionMatch = releaseContent.match(/JAVA_VERSION="(.+)"/);
                const javaVersion = versionMatch ? versionMatch[1].replace(/"/g, '') : 'Unknown';
                console.log(`  ${version} (current): Java ${javaVersion} (${fullJdkPath})`);
              } catch (e) {
                console.log(`  ${version} (${fullJdkPath})`);
              }
            }
          }
        });
      }
    } else if (args[1] === 'use' && args[2]) {
      const version = args[2];
      const versionDir = path.join(fnvaDir, version);

      if (!fs.existsSync(versionDir)) {
        console.error(`Java environment '${version}' not found`);
        process.exit(1);
      }

      // 查找实际的 JDK 目录
      const jdkSubdirs = fs.readdirSync(versionDir, { withFileTypes: true })
        .filter(dirent => dirent.isDirectory())
        .map(dirent => dirent.name);

      if (jdkSubdirs.length === 0) {
        console.error(`No JDK installation found in ${versionDir}`);
        process.exit(1);
      }

      const jdkPath = path.join(versionDir, jdkSubdirs[0]);
      const jdkBinPath = path.join(jdkPath, 'bin');

      // 生成环境切换脚本
      const envVars = {
        JAVA_HOME: jdkPath
      };

      const script = generateSimpleScript(envVars, 'java', version);
      console.log(script);
    } else {
      console.error('Usage: fnva java <list|use <version>>');
      process.exit(1);
    }
  } else {
    console.error(`❌ Command '${args[0]}' requires native binary mode`);
    console.error('');
    console.error('当前运行在 Node.js 降级模式，不支持此命令');
    console.error('');
    console.error('解决方案:');
    console.error('1. 重新安装 npm 包: npm install -g fnva --force');
    console.error('2. 从 GitHub Release 下载原生二进制文件');
    console.error('3. 或者设置 FNVA_SKIP_NATIVE=1 强制使用此模式（功能受限）');
    process.exit(1);
  }
}

function run() {
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
    if (process.env.FNVA_SKIP_NATIVE === '1') {
      if (showDebug) {
        console.log('Falling back to Node.js mode (FNVA_SKIP_NATIVE set)');
      }
      // 纯 Node.js 模式 - 实现基本的环境切换功能
      const args = process.argv.slice(2);
      handleNodeOnlyMode(args);
      return;
    }

    console.error('❌ Error: fnva native binary not found.');
    console.error('');

    if (showDebug) {
      console.error('🔍 Debug information is shown above');
      console.error('');
    }

    console.error("💡 Solutions:");
    console.error("  1) Reinstall npm package: npm install -g fnva --force");
    console.error("  2) Download binary from GitHub Release");
    console.error("  3) Set FNVA_SKIP_NATIVE=1 to use Node.js mode (limited functionality)");
    console.error("  4) Set FNVA_DEBUG=1 to show debug information");
    process.exit(1);
  }

  let args = process.argv.slice(2);

  // 如果设置了 FNVA_AUTO_EXECUTE，则为环境切换命令启用自动执行
  if (process.env.FNVA_AUTO_EXECUTE === '1' && isEnvironmentSwitchCommand(args) && !hasSessionFlag(args)) {
    // 添加 --auto 标志来启用自动执行
    args = args.concat('--auto');
  }
  const isSwitchCommand = isEnvironmentSwitchCommand(args);

  // Unix: 直接透传给 Rust 二进制，不拦截参数
  // shell wrapper 函数负责捕获输出并 source
  if (process.platform !== 'win32') {
    const result = spawnSync(binaryPath, args, {
      stdio: 'inherit',
    });

    if (result.error) {
      if (result.error.code === 'EACCES') {
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

  if (isSwitchCommand) {
    const shellArg = getShellArg(args);
    if (!shellArg || shellArg === 'auto') {
      const detected = detectShell();
      if (shellArg === 'auto') {
        const idx = args.indexOf('--shell');
        if (idx !== -1 && idx + 1 < args.length) {
          args[idx + 1] = detected;
        }
      } else {
        args.push('--shell', detected);
      }
    }

    const { spawnSync } = require('child_process');
    const result = spawnSync(binaryPath, args, {
      encoding: 'utf8',
      shell: false
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

    if (result.status !== 0) {
      process.exit(result.status);
    }

    // 获取环境切换脚本
    const stdout = result.stdout || '';
    if (stdout.includes('JAVA_HOME') || stdout.includes('ANTHROPIC_') || stdout.includes('OPENAI_')) {
      // 将数组输出转换为字符串
      const script = Array.isArray(stdout) ? stdout.join('\n') : stdout;
      const envType = args[0];
      const envName = args[2];

      // Windows：默认不启动新的会话；可通过 --session 开启旧行为
      if (process.platform === 'win32') {
        if (hasSessionFlag(args)) {
          console.log(`[OK] Switched to ${envType} environment: ${envName}`);
          console.log(`[INFO] Starting new PowerShell session with ${envName} environment...`);
          console.log(`Type "exit" to return to previous session\n`);

          try {
            const os = require('os');
            const tempScript = os.tmpdir() + '\\fnva_env_' + Date.now() + '.ps1';
            const fullScript = script + '\n';
            // 使用编码工具写入文件
            EncodingUtils.writeFileWithEncoding(tempScript, fullScript);
            const { spawn } = require('child_process');
            const ps = spawn('powershell', ['-NoExit', '-ExecutionPolicy', 'Bypass', '-File', tempScript], {
              stdio: 'inherit',
              shell: false
            });
            ps.on('exit', () => {
              try { fs.unlinkSync(tempScript); } catch (_) {}
              console.log('[INFO] Returned to original session');
            });
            return;
          } catch (error) {
            console.error(`Failed to start PowerShell session: ${error.message}`);
            console.log(`📝 Script was: ${script}`);
          }
        } else {
          // 检查是否使用了 --apply 参数
          if (hasApplyFlag(args)) {
            // 直接应用环境变量到当前进程
            const envVars = parseEnvironmentScript(script);
            applyEnvironmentVariables(envVars);
            displaySuccessMessage(envType, envName, envVars);
          } else {
            // 在 Windows 中，智能处理环境设置
            const envVars = parseEnvironmentScript(script);
            const simpleScript = generateSimpleScript(envVars, envType, envName);

            // 尝试自动执行（如果可能）
            if (process.env.FNVA_AUTO_EXECUTE === '1') {
              const os = require('os');
              const fs = require('fs');
              const path = require('path');
              const { spawn } = require('child_process');

              try {
                const tempFile = path.join(os.tmpdir(), `fnva_auto_${Date.now()}.ps1`);
                // 使用编码工具写入文件
                EncodingUtils.writeFileWithEncoding(tempFile, simpleScript);

                // 使用 PowerShell 执行脚本
                spawn('powershell', ['-ExecutionPolicy', 'Bypass', '-File', tempFile], {
                  stdio: 'inherit',
                  shell: false
                }).on('exit', () => {
                  try { fs.unlinkSync(tempFile); } catch (_) {}
                });

                console.log('[OK] 环境已自动切换');
                return;
              } catch (error) {
                console.warn('[WARN] 自动执行失败，回退到脚本输出');
              }
            }

            // 默认输出脚本
            process.stdout.write(simpleScript);
          }
        }
      } else {
        // Unix-like systems: output raw script for eval or wrapper function
        process.stdout.write(script);
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
    // For env commands, capture output and write as single string to avoid
    // PowerShell splitting multi-line output into Object[] (which breaks
    // the standard | Out-String | Invoke-Expression profile pattern).
    const isEnvOutputCommand = args[0] === 'env' && args[1] === 'env';
    if (isEnvOutputCommand) {
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

    // 对于其他命令，使用原有的 stdio: 'inherit' 方式
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
}

run();

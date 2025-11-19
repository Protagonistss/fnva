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
  // 如果设置了 FNVA_SKIP_NATIVE，跳过原生二进制查找
  if (process.env.FNVA_SKIP_NATIVE === '1') {
    return null;
  }

  // 如果设置了 FNVA_AUTO_MODE，自动使用 Node.js 模式
  if (process.env.FNVA_AUTO_MODE === '1') {
    return null;
  }

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

function hasDirectExecuteFlag(args) {
  return args.includes('--exec') || args.includes('-e');
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

function hasAutoExecuteFlag(args) {
  return args.includes('--auto');
}

function removeAutoFlag(args) {
  const index = args.indexOf('--auto');
  if (index > -1) {
    return args.slice(0, index).concat(args.slice(index + 1));
  }
  return args;
}

function createTempScriptFile(script, envType, envName) {
  try {
    const os = require('os');
    const fs = require('fs');
    const path = require('path');

    const tempDir = os.tmpdir();
    const scriptFile = path.join(tempDir, `fnva_${envType}_${envName}_${Date.now()}.ps1`);

    fs.writeFileSync(scriptFile, script, 'utf8');

    console.log('');
    console.log('💡 环境已切换到当前进程。要在新的 PowerShell 窗口中使用此环境，运行：');
    console.log(`   ${scriptFile}`);
    console.log('   或者: fnva', envType, 'use', envName, '--auto');

  } catch (error) {
    console.warn('⚠️  无法创建临时脚本文件:', error.message);
  }
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

function generateSimpleScript(envVars, envType, envName) {
  const lines = [];

  if (process.platform === 'win32') {
    // Windows PowerShell
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

  // 简单的命令处理
  if (args.length === 0) {
    console.log('fnva - 环境管理工具 (Node.js 模式)');
    console.log('');
    console.log('支持的命令:');
    console.log('  java list     - 列出 Java 环境');
    console.log('  java use <n>  - 切换 Java 环境');
    console.log('');
    console.log('注意: Node.js 模式功能有限，建议使用原生二进制版本。');
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
    console.error(`Command '${args[0]}' not supported in Node.js mode`);
    process.exit(1);
  }
}

function run() {
  const binaryPath = buildBinaryPath();

  if (!binaryPath) {
    if (process.env.FNVA_SKIP_NATIVE === '1' || process.env.FNVA_AUTO_MODE === '1') {
      // 纯 Node.js 模式 - 实现基本的环境切换功能
      const args = process.argv.slice(2);
      handleNodeOnlyMode(args);
      return;
    }

    console.error('Error: fnva native binary not found.');
    console.error('');
    console.error("Please either:");
    console.error("  1) Run 'npm run build' (or 'npm run build:all') to produce platform binaries,");
    console.error("  2) Install a release package that includes the platforms directory, or");
    console.error("  3) Set FNVA_NATIVE_PATH to the full path of an existing fnva executable.");
    console.error("  4) Set FNVA_SKIP_NATIVE=1 to use Node.js mode (limited functionality).");
    process.exit(1);
  }

  let args = process.argv.slice(2);

  // 如果设置了 FNVA_AUTO_EXECUTE，则为环境切换命令启用自动执行
  if (process.env.FNVA_AUTO_EXECUTE === '1' && isEnvironmentSwitchCommand(args) && !hasSessionFlag(args)) {
    // 添加 --auto 标志来启用自动执行
    args = args.concat('--auto');
  }
  const isSwitchCommand = isEnvironmentSwitchCommand(args);

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
      const envType = args[0];
      const envName = args[2];

      // Windows：默认不启动新的会话；可通过 --session 开启旧行为
      if (process.platform === 'win32') {
        if (hasSessionFlag(args)) {
          console.log(`✅ Switched to ${envType} environment: ${envName}`);
          console.log(`🚀 Starting new PowerShell session with ${envName} environment...`);
          console.log(`Type "exit" to return to previous session\n`);

          try {
            const os = require('os');
            const fs = require('fs');
            const tempScript = os.tmpdir() + '\\fnva_env_' + Date.now() + '.ps1';
            const fullScript = script + '\n';
            fs.writeFileSync(tempScript, fullScript, 'utf8');
            const { spawn } = require('child_process');
            const ps = spawn('powershell', ['-NoExit', '-ExecutionPolicy', 'Bypass', '-File', tempScript], {
              stdio: 'inherit',
              shell: false
            });
            ps.on('exit', () => {
              try { fs.unlinkSync(tempScript); } catch (_) {}
              console.log('👋 Returned to original session');
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
                fs.writeFileSync(tempFile, simpleScript, 'utf8');

                // 使用 PowerShell 执行脚本
                spawn('powershell', ['-ExecutionPolicy', 'Bypass', '-File', tempFile], {
                  stdio: 'inherit',
                  shell: false
                }).on('exit', () => {
                  try { fs.unlinkSync(tempFile); } catch (_) {}
                });

                console.log('✅ 环境已自动切换');
                return;
              } catch (error) {
                console.warn('⚠️  自动执行失败，回退到脚本输出');
              }
            }

            // 默认输出脚本
            process.stdout.write(simpleScript);
          }
        }
      } else {
        // Unix-like systems: 显示使用说明
        console.log(`✅ Switched to ${envType} environment: ${envName}`);
        console.log('');
        console.log('💡 To apply this environment, run:');
        console.log(`  node bin/fnva.js ${args.join(' ')} | bash`);
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

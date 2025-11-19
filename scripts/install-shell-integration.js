#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const os = require('os');
const { spawn } = require('child_process');

function detectShell() {
  if (process.platform === 'win32') {
    return 'powershell';
  }
  return process.env.SHELL?.split('/').pop() || 'bash';
}

function getShellConfigPath(shell) {
  switch (shell) {
    case 'powershell':
      return path.join(process.env.USERPROFILE || os.homedir(), 'Documents', 'WindowsPowerShell', 'Microsoft.PowerShell_profile.ps1');
    case 'bash':
      return path.join(os.homedir(), '.bashrc');
    case 'zsh':
      return path.join(os.homedir(), '.zshrc');
    case 'fish':
      return path.join(os.homedir(), '.config', 'fish', 'config.fish');
    default:
      return null;
  }
}

function getPowerShellFunction() {
  return `
# fnva 自动化函数 - 由 npm 安装自动添加
function fnva {
    if ($args.Count -ge 2 -and ($args[0] -eq "java" -or $args[0] -eq "llm" -or $args[0] -eq "cc") -and ($args[1] -eq "use")) {
        $tempFile = Join-Path $env:TEMP ("fnva_script_" + (Get-Random) + ".ps1")

        $env:FNVAAUTOMODE = "1"
        try {
            # 捕获 fnva 输出并保存到临时文件
            $output = cmd.exe /c "set FNVA_AUTO_MODE=%FNVAAUTOMODE% && fnva $args" 2>&1

            # 如果输出包含 PowerShell 脚本内容，保存并执行
            if ($output -match '\$env:' -or $output -match 'Write-Host') {
                $output | Out-File -FilePath $tempFile -Encoding UTF8
                try {
                    & $tempFile
                } catch {
                    Write-Host "执行脚本时出错: $_" -ForegroundColor Red
                }
            } else {
                # 如果不是脚本内容，直接输出
                $output
            }
        } finally {
            $env:FNVAAUTOMODE = ""
            if (Test-Path $tempFile) {
                Remove-Item $tempFile -ErrorAction SilentlyContinue
            }
        }
    } else {
        $env:FNVAAUTOMODE = "1"
        try {
            cmd.exe /c "set FNVA_AUTO_MODE=%FNVAAUTOMODE% && fnva $args"
        } finally {
            $env:FNVAAUTOMODE = ""
        }
    }
}
`;
}

function getBashFunction() {
  return `
# fnva 自动化函数 - 由 npm 安装自动添加
fnva() {
    if [[ \$# -ge 2 && ("\$1" == "java" || "\$1" == "llm" || "\$1" == "cc") && "\$2" == "use" ]]; then
        local temp_file=\$(mktemp)
        chmod +x "\$temp_file"

        FNVA_AUTO_MODE=1 fnva "\$@" > "\$temp_file"
        source "\$temp_file"
        rm -f "\$temp_file"
    else
        FNVA_AUTO_MODE=1 fnva "\$@"
    fi
}
`;
}

function getFishFunction() {
  return `
# fnva 自动化函数 - 由 npm 安装自动添加
function fnva
    if test (count \$argv) -ge 2; and string match -q -r "^(java|llm|cc)\$" \$argv[1]; and test \$argv[2] = "use"
        set temp_file (mktemp)
        chmod +x \$temp_file
        env FNVA_AUTO_MODE=1 fnva \$argv > \$temp_file
        source \$temp_file
        rm -f \$temp_file
    else
        env FNVA_AUTO_MODE=1 fnva \$argv
    end
end
`;
}

function getShellFunction(shell) {
  switch (shell) {
    case 'powershell':
      return getPowerShellFunction();
    case 'bash':
      return getBashFunction();
    case 'zsh':
      return getBashFunction(); // zsh 使用和 bash 相同的语法
    case 'fish':
      return getFishFunction();
    default:
      return '';
  }
}

function isFunctionInstalled(configPath, shell) {
  if (!fs.existsSync(configPath)) {
    return false;
  }

  const content = fs.readFileSync(configPath, 'utf8');
  return content.includes('fnva 自动化函数 - 由 npm 安装自动添加');
}

function installShellIntegration() {
  const shell = detectShell();
  const configPath = getShellConfigPath(shell);

  if (!configPath) {
    console.log(`❌ 不支持的 shell: ${shell}`);
    console.log('请手动配置 fnva，详见: https://github.com/your-repo/fnva');
    return false;
  }

  if (isFunctionInstalled(configPath, shell)) {
    console.log(`✅ fnva shell 集成已安装在: ${configPath}`);
    return true;
  }

  try {
    // 确保目录存在
    const dir = path.dirname(configPath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }

    // 获取函数定义
    const functionCode = getShellFunction(shell);

    // 添加到配置文件
    if (fs.existsSync(configPath)) {
      const content = fs.readFileSync(configPath, 'utf8');
      fs.writeFileSync(configPath, content + '\n' + functionCode);
    } else {
      fs.writeFileSync(configPath, functionCode);
    }

    console.log(`✅ fnva shell 集成已安装到: ${configPath}`);
    console.log('🔄 请重新加载你的 shell 配置:');

    switch (shell) {
      case 'powershell':
        console.log('   . $PROFILE');
        break;
      case 'bash':
        console.log('   source ~/.bashrc');
        break;
      case 'zsh':
        console.log('   source ~/.zshrc');
        break;
      case 'fish':
        console.log('   source ~/.config/fish/config.fish');
        break;
    }

    return true;
  } catch (error) {
    console.log(`❌ 安装失败: ${error.message}`);
    console.log('请手动配置 fnva');
    return false;
  }
}

// 询问用户是否安装
function promptInstallation() {
  if (process.env.FNVA_SKIP_SHELL_SETUP === '1') {
    console.log('⏭️  跳过 shell 集成安装');
    return;
  }

  const shell = detectShell();
  console.log(`🔧 检测到 shell: ${shell}`);
  console.log('🚀 是否安装 fnva shell 集成? (y/N)');

  process.stdin.resume();
  process.stdin.setEncoding('utf8');

  process.stdin.on('data', function(data) {
    const response = data.toString().trim().toLowerCase();
    if (response === 'y' || response === 'yes') {
      installShellIntegration();
    } else {
      console.log('⏭️  跳过 shell 集成安装');
      console.log('📖 手动配置指南: https://github.com/your-repo/fnva');
    }
    process.exit(0);
  });

  // 10秒后自动跳过
  setTimeout(() => {
    console.log('⏭️  超时，跳过 shell 集成安装');
    console.log('📖 手动配置指南: https://github.com/your-repo/fnva');
    process.exit(0);
  }, 10000);
}

// 主程序
if (require.main === module) {
  console.log('🔧 fnva shell 集成安装器');
  console.log(`📍 Node.js 进程ID: ${process.pid}`);
  console.log(`📂 工作目录: ${process.cwd()}`);
  console.log(`🎯 参数: ${process.argv.join(' ')}`);

  if (process.argv.includes('--auto') || process.argv.includes('--yes')) {
    console.log('🚀 自动模式启动安装...');
    const result = installShellIntegration();
    console.log(`🏁 安装结果: ${result ? '成功' : '失败'}`);
  } else {
    promptInstallation();
  }
}

module.exports = {
  detectShell,
  getShellConfigPath,
  getShellFunction,
  isFunctionInstalled,
  installShellIntegration
};
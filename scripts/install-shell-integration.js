#!/usr/bin/env node

// Shell integration installer for fnva (npm postinstall helper).
// Generates a lightweight function wrapper per shell that calls the real
// `fnva` binary and applies environment switch scripts.

const fs = require('fs');
const path = require('path');
const os = require('os');

function detectShell() {
  if (process.platform === 'win32') {
    return 'powershell';
  }
  return process.env.SHELL?.split('/').pop() || 'bash';
}

function getShellConfigPath(shell) {
  switch (shell) {
    case 'powershell':
      return path.join(
        process.env.USERPROFILE || os.homedir(),
        'Documents',
        'WindowsPowerShell',
        'Microsoft.PowerShell_profile.ps1'
      );
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
# fnva auto integration (added by npm install)
function fnva {
    if ($args.Count -ge 2 -and ($args[0] -eq "java" -or $args[0] -eq "llm" -or $args[0] -eq "cc") -and ($args[1] -eq "use")) {
        $tempFile = Join-Path $env:TEMP ("fnva_script_" + (Get-Random) + ".ps1")

        $env:FNVAAUTOMODE = "1"
        try {
            $output = cmd.exe /c "set FNVA_AUTO_MODE=%FNVAAUTOMODE% && fnva $args" 2>&1

            if ($output -match '\\$env:' -or $output -match 'Write-Host') {
                $output | Out-File -FilePath $tempFile -Encoding UTF8
                try { & $tempFile } catch { Write-Host "Error executing script: $_" -ForegroundColor Red }
            } else {
                $output
            }
        } finally {
            $env:FNVAAUTOMODE = ""
            if (Test-Path $tempFile) { Remove-Item $tempFile -ErrorAction SilentlyContinue }
        }
    } else {
        $env:FNVAAUTOMODE = "1"
        try { cmd.exe /c "set FNVA_AUTO_MODE=%FNVAAUTOMODE% && fnva $args" } finally { $env:FNVAAUTOMODE = "" }
    }
}
`;
}

function getBashFunction() {
  return `
# fnva auto integration (added by npm install)
fnva() {
    local __fnva_bin
    __fnva_bin="$(command -v fnva | head -n 1)"
    if [[ -z "$__fnva_bin" ]]; then
        echo "fnva: binary not found in PATH" >&2
        return 127
    fi

    if [[ $# -ge 2 && ("$1" == "java" || "$1" == "llm" || "$1" == "cc") && "$2" == "use" ]]; then
        local temp_file
        temp_file="$(mktemp)"
        chmod +x "$temp_file"

        FNVA_AUTO_MODE=1 "$__fnva_bin" "$@" > "$temp_file"
        source "$temp_file"
        rm -f "$temp_file"
    else
        FNVA_AUTO_MODE=1 "$__fnva_bin" "$@"
    fi
}
`;
}

function getFishFunction() {
  return `
# fnva auto integration (added by npm install)
function fnva
    set __fnva_bin (command -v fnva | head -n 1)
    if test -z "$__fnva_bin"
        echo "fnva: binary not found in PATH" >&2
        return 127
    end

    if test (count $argv) -ge 2; and string match -q -r "^(java|llm|cc)$" $argv[1]; and test $argv[2] = "use"
        set temp_file (mktemp)
        chmod +x $temp_file
        env FNVA_AUTO_MODE=1 "$__fnva_bin" $argv > $temp_file
        source $temp_file
        rm -f $temp_file
    else
        env FNVA_AUTO_MODE=1 "$__fnva_bin" $argv
    end
end
`;
}

function getShellFunction(shell) {
  switch (shell) {
    case 'powershell':
      return getPowerShellFunction();
    case 'bash':
    case 'zsh':
      return getBashFunction();
    case 'fish':
      return getFishFunction();
    default:
      return '';
  }
}

function isFunctionInstalled(configPath) {
  if (!fs.existsSync(configPath)) {
    return false;
  }
  const content = fs.readFileSync(configPath, 'utf8');
  return content.includes('fnva auto integration (added by npm install)');
}

function installShellIntegration() {
  const shell = detectShell();
  const configPath = getShellConfigPath(shell);

  if (!configPath) {
    console.log(`⚠️  Unsupported shell: ${shell}`);
    console.log('Please configure fnva manually (see README).');
    return false;
  }

  if (isFunctionInstalled(configPath)) {
    console.log(`✅ fnva shell integration already present: ${configPath}`);
    return true;
  }

  try {
    const dir = path.dirname(configPath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }

    const functionCode = getShellFunction(shell);

    if (fs.existsSync(configPath)) {
      const content = fs.readFileSync(configPath, 'utf8');
      fs.writeFileSync(configPath, content + '\n' + functionCode);
    } else {
      fs.writeFileSync(configPath, functionCode);
    }

    console.log(`✅ fnva shell integration installed at: ${configPath}`);
    console.log('🔄 Reload your shell config:');
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
    console.log(`❌ Install failed: ${error.message}`);
    console.log('Please configure fnva manually (see README).');
    return false;
  }
}

function promptInstallation() {
  if (process.env.FNVA_SKIP_SHELL_SETUP === '1') {
    console.log('⏭️  Skipping shell integration (FNVA_SKIP_SHELL_SETUP=1)');
    return;
  }

  const shell = detectShell();
  console.log(`🔍 Detected shell: ${shell}`);
  console.log('❓ Install fnva shell integration? (y/N)');

  const readline = require('readline');
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  rl.question('> ', (answer) => {
    const normalized = answer.trim().toLowerCase();
    if (normalized === 'y' || normalized === 'yes') {
      installShellIntegration();
    } else {
      console.log('⏩ Skipped shell integration.');
    }
    rl.close();
  });
}

function main() {
  console.log('🛠️ fnva shell integration installer');
  console.log(`📦 Node.js version: ${process.version}`);
  console.log(`📍 CWD: ${process.cwd()}`);

  if (process.argv.includes('--auto') || process.argv.includes('--yes')) {
    console.log('🤖 Auto mode: installing...');
    const result = installShellIntegration();
    console.log(`📄 Install result: ${result ? 'success' : 'failed'}`);
  } else {
    promptInstallation();
  }
}

if (require.main === module) {
  main();
}

module.exports = {
  detectShell,
  getShellConfigPath,
  getShellFunction,
  isFunctionInstalled,
  installShellIntegration,
};

#!/usr/bin/env node

// Shell integration installer for fnva (npm postinstall helper).
// Uses fnva's built-in template system for shell integration scripts.

const fs = require('fs');
const path = require('path');
const os = require('os');
const { spawnSync } = require('child_process');

// Absolute path to the packaged fnva shim (bin/fnva.js)
const FNVA_SHIM = path.resolve(__dirname, '..', 'bin', 'fnva.js');

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

// Get integration script from fnva command (uses Rust templates)
function getIntegrationScript(shell) {
  try {
    const result = spawnSync('node', [FNVA_SHIM, 'env', 'shell-integration', '-s', shell], {
      encoding: 'utf8',
      cwd: path.resolve(__dirname, '..')
    });

    if (result.status === 0 && result.stdout) {
      return result.stdout;
    }

    console.log(`Warning: Failed to get integration script from fnva (exit code: ${result.status})`);
    if (result.stderr) {
      console.log(`stderr: ${result.stderr}`);
    }
    return '';
  } catch (error) {
    console.log(`Warning: Failed to call fnva for integration script: ${error.message}`);
    return '';
  }
}

function getPowerShellFunction() {
  // Get integration script from Rust template
  const integrationScript = getIntegrationScript('powershell');

  return `
${integrationScript}

# fnva auto integration (added by npm install)
function fnva {
    if ($args.Count -ge 2 -and ($args[0] -eq "java" -or $args[0] -eq "llm" -or $args[0] -eq "cc") -and ($args[1] -eq "use")) {
        $tempFile = Join-Path $env:TEMP ("fnva_script_" + (Get-Random) + ".ps1")

        try {
            # Directly pipe output to temp file to avoid encoding issues
            & fnva.cmd @args 2>&1 | Out-File -FilePath $tempFile -Encoding UTF8

            # Check if file contains environment variables
            $content = Get-Content $tempFile -Raw -Encoding UTF8
            if ($content -match '\\$env:' -or $content -match 'Write-Host') {
                # Use dot sourcing to execute in current scope
                . $tempFile
            } else {
                $content
            }
        } finally {
            if (Test-Path $tempFile) { Remove-Item $tempFile -ErrorAction SilentlyContinue }
        }
    } else {
        & fnva.cmd @args
    }
}
`;
}

function getBashFunction() {
  // Get integration script from Rust template
  const integrationScript = getIntegrationScript('bash');

  return `
${integrationScript}

# fnva auto integration (added by npm install)
fnva() {
    local __fnva_bin="${FNVA_SHIM}"
    if [[ ! -x "$__fnva_bin" ]]; then
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
  // Get integration script from Rust template
  const integrationScript = getIntegrationScript('fish');

  return `
${integrationScript}

# fnva auto integration (added by npm install)
function fnva
    set __fnva_bin "${FNVA_SHIM}"
    if test ! -x "$__fnva_bin"
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
      return getBashFunction(); // zsh 使用和 bash 相同的语法
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
    console.log(`Unsupported shell: ${shell}`);
    console.log('Please configure fnva manually (see README).');
    return false;
  }

  if (isFunctionInstalled(configPath)) {
    console.log(`fnva shell integration already present: ${configPath}`);
    return true;
  }

  try {
    const dir = path.dirname(configPath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }

    const functionCode = getShellFunction(shell);

    if (!functionCode) {
      console.log('Failed to generate shell integration script');
      return false;
    }

    if (fs.existsSync(configPath)) {
      const content = fs.readFileSync(configPath, 'utf8');
      fs.writeFileSync(configPath, content + '\n' + functionCode);
    } else {
      fs.writeFileSync(configPath, functionCode);
    }

    console.log(`fnva shell integration installed at: ${configPath}`);
    console.log('Reload your shell config after install.');
    return true;
  } catch (error) {
    console.log(`Install failed: ${error.message}`);
    console.log('Please configure fnva manually (see README).');
    return false;
  }
}

function promptInstallation() {
  if (process.env.FNVA_SKIP_SHELL_SETUP === '1') {
    console.log('Skipping shell integration (FNVA_SKIP_SHELL_SETUP=1)');
    return;
  }

  const shell = detectShell();
  console.log(`Detected shell: ${shell}`);
  console.log('Install fnva shell integration? (y/N)');

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
      console.log('Skipped shell integration.');
    }
    rl.close();
  });
}

function main() {
  console.log('fnva shell integration installer');
  console.log(`Node.js version: ${process.version}`);
  console.log(`CWD: ${process.cwd()}`);

  if (process.argv.includes('--auto') || process.argv.includes('--yes')) {
    const result = installShellIntegration();
    console.log(`Install result: ${result ? 'success' : 'failed'}`);
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

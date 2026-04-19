#!/usr/bin/env node

// Shell integration installer for fnva (npm postinstall helper).
// Writes a single line into the shell profile: eval "$(fnva env --shell <shell>)"
// The fnva binary generates the full init script (autoload + wrapper).

const fs = require('fs');
const path = require('path');
const os = require('os');
const { spawnSync } = require('child_process');

const FNVA_SHIM = path.resolve(__dirname, '..', 'bin', 'fnva.js');

function detectShell() {
  if (process.platform === 'win32') {
    return 'powershell';
  }
  return process.env.SHELL?.split('/').pop() || 'bash';
}

function getPowershellProfilePath() {
  const home = process.env.USERPROFILE || os.homedir();
  // Check PowerShell 7 first (pwsh), then fall back to Windows PowerShell 5.x
  const ps7Profile = path.join(home, 'Documents', 'PowerShell', 'Microsoft.PowerShell_profile.ps1');
  const ps5Profile = path.join(home, 'Documents', 'WindowsPowerShell', 'Microsoft.PowerShell_profile.ps1');
  // If PS7 profile already exists, use it; otherwise prefer PS7 path
  if (fs.existsSync(ps7Profile)) return ps7Profile;
  if (fs.existsSync(ps5Profile)) return ps5Profile;
  // Detect which PowerShell is installed via pwsh vs powershell
  const pwshResult = spawnSync('pwsh', ['-NoProfile', '-Command', 'echo true'], { timeout: 5000 });
  if (pwshResult.status === 0) return ps7Profile;
  return ps5Profile;
}

function getShellConfigPath(shell) {
  switch (shell) {
    case 'powershell':
      return getPowershellProfilePath();
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

function getIntegrationLine(shell) {
  switch (shell) {
    case 'powershell':
      return 'Invoke-Expression (& fnva env env --shell powershell | Out-String)';
    case 'bash':
    case 'zsh':
      return 'eval "$(fnva env env --shell bash)"';
    case 'fish':
      return 'fnva env env --shell fish | source';
    default:
      return null;
  }
}

function isInstalled(configPath) {
  if (!fs.existsSync(configPath)) {
    return false;
  }
  const content = fs.readFileSync(configPath, 'utf8');
  return content.includes('fnva env env --shell');
}

function installShellIntegration() {
  const shell = detectShell();
  const configPath = getShellConfigPath(shell);

  if (!configPath) {
    console.log(`Unsupported shell: ${shell}`);
    console.log('Please configure fnva manually (see README).');
    return false;
  }

  if (isInstalled(configPath)) {
    console.log(`fnva shell integration already present: ${configPath}`);
    return true;
  }

  const line = getIntegrationLine(shell);
  if (!line) {
    console.log(`Unsupported shell: ${shell}`);
    return false;
  }

  try {
    const dir = path.dirname(configPath);
    if (!fs.existsSync(dir)) {
      fs.mkdirSync(dir, { recursive: true });
    }

    const marker = `\n# fnva shell integration\n${line}\n`;

    if (fs.existsSync(configPath)) {
      const content = fs.readFileSync(configPath, 'utf8');
      fs.writeFileSync(configPath, content + marker);
    } else {
      fs.writeFileSync(configPath, marker);
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
  getIntegrationLine,
  isInstalled,
  installShellIntegration,
};

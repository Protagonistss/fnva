#!/usr/bin/env node

// Shell integration uninstaller for fnva (npm postuninstall helper).
// Cleans fnva wrapper functions from common shell config files.

const fs = require('fs');
const path = require('path');
const os = require('os');

function detectShell() {
  if (process.platform === 'win32') return 'powershell';
  return process.env.SHELL?.split('/').pop() || 'bash';
}

function getShellConfigPaths(shell) {
  const home = process.env.USERPROFILE || os.homedir();
  switch (shell) {
    case 'powershell':
      // Cover both PowerShell 7 (PowerShell\) and Windows PowerShell 5.x (WindowsPowerShell\).
      return [
        path.join(home, 'Documents', 'PowerShell', 'Microsoft.PowerShell_profile.ps1'),
        path.join(home, 'Documents', 'WindowsPowerShell', 'Microsoft.PowerShell_profile.ps1'),
      ];
    case 'bash':
      return [path.join(os.homedir(), '.bashrc')];
    case 'zsh':
      return [
        path.join(os.homedir(), '.zshrc'),
        path.join(os.homedir(), '.oh-my-zsh', 'custom', '.zshrc'),
      ];
    case 'fish':
      return [path.join(os.homedir(), '.config', 'fish', 'config.fish')];
    default:
      return [];
  }
}

function cleanConfigFile(cfgPath) {
  const original = fs.readFileSync(cfgPath, 'utf8');
  let content = original;

  // Current fenced marker block — matches what install-shell-integration.js
  // and install.ps1 write. Same marker the irm uninstaller strips.
  content = content.replace(/\r?\n?# >>> fnva >>>[\s\S]*?# <<< fnva <<</g, '');

  // Legacy one-line marker written by fnva 0.0.75–0.0.86.
  content = content.replace(/\r?\n?# fnva shell integration\r?\n[^\r\n]*(?:\r?\n)?/g, '');

  if (content === original) {
    console.log(`⚠️  No fnva block found in ${cfgPath}`);
    return false;
  }

  content = content.replace(/\n{3,}/g, '\n\n').trim() + '\n';
  fs.writeFileSync(cfgPath, content);
  console.log(`✅ fnva shell integration removed from ${cfgPath}`);
  return true;
}

// Older fnva versions copied bin/fnva.ps1 into what they guessed was the npm
// global bin dir. On Windows + recent Node that guess fell back to the
// node.exe directory, leaving a stray fnva.ps1 on PATH that shadowed the real
// binary and produced "native binary not found". Remove it (Windows only,
// best-effort). Never touch the real npm prefix — npm owns fnva.ps1 there.
function cleanupLegacyPs1() {
  if (process.platform !== 'win32') return;
  try {
    const nodeDir = path.dirname(process.execPath);
    const npmPrefix = process.env.npm_config_prefix || '';
    if (npmPrefix && path.resolve(nodeDir) === path.resolve(npmPrefix)) return;
    const stray = path.join(nodeDir, 'fnva.ps1');
    if (fs.existsSync(stray)) {
      fs.unlinkSync(stray);
      console.log(`🧹 Removed legacy fnva.ps1 shim from ${stray}`);
    }
  } catch (_) {
    // best-effort; ignore
  }
}

function removeShellIntegration(configPath, shell) {
  const paths = configPath ? [configPath] : getShellConfigPaths(shell);
  let removedAny = false;

  for (const cfgPath of paths) {
    if (!cfgPath || !fs.existsSync(cfgPath)) continue;
    try {
      const removed = cleanConfigFile(cfgPath);
      removedAny = removedAny || removed;
    } catch (error) {
      console.log(`❌ Remove failed (${cfgPath}): ${error.message}`);
    }
  }

  if (!removedAny) {
    console.log('⚠️  No shell config cleaned (file missing or no fnva block)');
  }

  return removedAny;
}

function main() {
  console.log('🧹 fnva shell integration uninstaller');
  console.log('npm install location:', __dirname);
  console.log('Current platform:', process.platform);
  console.log('Detected shell:', process.env.SHELL || 'unknown');

  // Best-effort: remove any stray fnva.ps1 the old installer dropped on PATH.
  cleanupLegacyPs1();

  const shell = detectShell();
  const paths = getShellConfigPaths(shell);

  console.log(`Config paths for ${shell}:`, paths);

  if (!paths.length) {
    console.log(`⚠️  Unsupported shell: ${shell}`);
    console.log('No config files found for this shell');
    return;
  }

  console.log(`Attempting to clean config files...`);
  const success = removeShellIntegration(null, shell);

  if (success) {
    console.log('✅ Shell integration successfully removed');
    console.log('🔄 Reload your shell config:');
    switch (shell) {
      case 'powershell':
        console.log('   . $PROFILE');
        console.log('   Or start new PowerShell instance');
        break;
      case 'bash':
        console.log('   source ~/.bashrc');
        console.log('   Or: exec bash');
        break;
      case 'zsh':
        console.log('   source ~/.zshrc');
        console.log('   Or: exec zsh');
        break;
      case 'fish':
        console.log('   source ~/.config/fish/config.fish');
        console.log('   Or: exec fish');
        break;
    }
  } else {
    console.log('⚠️  No fnva shell integration found in any config files');
    console.log('   (This is normal if shell integration was never installed)');
  }
}

if (require.main === module) {
  main();
}

module.exports = {
  detectShell,
  getShellConfigPaths,
  removeShellIntegration,
  cleanupLegacyPs1,
};
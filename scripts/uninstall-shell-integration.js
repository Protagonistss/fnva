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
  switch (shell) {
    case 'powershell':
      return [path.join(process.env.USERPROFILE || os.homedir(), 'Documents', 'WindowsPowerShell', 'Microsoft.PowerShell_profile.ps1')];
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
  let content = fs.readFileSync(cfgPath, 'utf8');
  const originalContent = content;

  const marker = '# fnva auto integration (added by npm install)';
  const startIndex = content.indexOf(marker);

  if (startIndex !== -1) {
    const beforeMarker = content.substring(0, startIndex).trimEnd();
    const afterMarker = content.substring(startIndex);
    const lines = afterMarker.split('\n');

    let functionEndIndex = -1;
    let braceCount = 0;
    let foundFunction = false;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      if (line.includes('function fnva') || line.includes('fnva(')) {
        foundFunction = true;
      }

      if (foundFunction) {
        for (const char of line) {
          if (char === '{') braceCount++;
          if (char === '}') braceCount--;
        }
        if (braceCount === 0) {
          functionEndIndex = i + 1;
          break;
        }
      }
    }

    if (functionEndIndex !== -1) {
      const afterFunction = lines.slice(functionEndIndex).join('\n');
      content = beforeMarker + '\n' + afterFunction;
    }
  }

  if (content === originalContent) {
    content = content
      .replace(/# fnva auto integration \(added by npm install\)[\s\S]*?(?=\n\S|\n$)/g, '')
      .replace(/.*fnva.*\n?/g, '')
      .replace(/.*FNVAAUTOMODE.*\n?/g, '')
      .replace(/.*cmd\.exe.*fnva.*\n?/g, '')
      .replace(/\n{3,}/g, '\n\n')
      .trim() + '\n';
  }

  if (content !== originalContent) {
    fs.writeFileSync(cfgPath, content);
    console.log(`✅ fnva shell integration removed from ${cfgPath}`);
    return true;
  }

  console.log(`⚠️  No fnva block found in ${cfgPath}`);
  return false;
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
};
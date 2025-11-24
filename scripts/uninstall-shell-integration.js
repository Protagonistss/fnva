#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const os = require('os');

function detectShell() {
  if (process.platform === 'win32') {
    return 'powershell';
  }
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

  const marker = '# fnva 自动化函数 - 用 npm 安装自动添加';
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

  // 正则兜底：移除残留 fnva 片段
  if (content === originalContent) {
    content = content
      .replace(/# fnva 自动化函数 - 用 npm 安装自动添加[\s\S]*?(?=\n\S|\n$)/g, '')
      .replace(/.*fnva.*\n?/g, '')
      .replace(/.*FNVAAUTOMODE.*\n?/g, '')
      .replace(/.*cmd\.exe.*fnva.*\n?/g, '')
      .replace(/\n{3,}/g, '\n\n')
      .trim() + '\n';
  }

  if (content !== originalContent) {
    fs.writeFileSync(cfgPath, content);
    console.log(`✅ fnva shell 集成已从 ${cfgPath} 移除`);
    return true;
  }

  console.log(`⚠️  未在 ${cfgPath} 找到需要清理的内容`);
  return false;
}

function removeShellIntegration(configPath, shell) {
  const paths = getShellConfigPaths(shell);
  if (configPath) paths.unshift(configPath); // 兼容传入单一路径

  let removedAny = false;
  for (const cfgPath of paths) {
    if (!cfgPath || !fs.existsSync(cfgPath)) continue;
    try {
      const removed = cleanConfigFile(cfgPath);
      removedAny = removedAny || removed;
    } catch (error) {
      console.log(`❌ 移除失败 (${cfgPath}): ${error.message}`);
    }
  }

  if (!removedAny) {
    console.log('⚠️  未找到可清理的 shell 配置文件或未匹配到 fnva 片段');
  }
  return removedAny;
}

function main() {
  console.log('🧹 fnva shell 集成卸载');

  const shell = detectShell();
  const paths = getShellConfigPaths(shell);

  if (paths.length === 0) {
    console.log(`⚠️  不支持的 shell: ${shell}`);
    return;
  }

  const success = removeShellIntegration(null, shell);

  if (success) {
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

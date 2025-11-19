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

function removeShellIntegration(configPath, shell) {
  if (!fs.existsSync(configPath)) {
    console.log(`⚠️  配置文件不存在: ${configPath}`);
    return false;
  }

  try {
    let content = fs.readFileSync(configPath, 'utf8');
    const originalContent = content;

    // 方法1: 查找标记，精确删除整个函数块
    const marker = '# fnva 自动化函数 - 由 npm 安装自动添加';
    const startIndex = content.indexOf(marker);

    if (startIndex !== -1) {
      // 找到标记前的换行符
      const beforeMarker = content.substring(0, startIndex).trimEnd();

      // 从标记开始查找完整的函数
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
          // 计算大括号
          for (const char of line) {
            if (char === '{') braceCount++;
            if (char === '}') braceCount--;
          }

          // 当大括号平衡时，函数结束
          if (braceCount === 0) {
            functionEndIndex = i + 1;
            break;
          }
        }
      }

      if (functionEndIndex !== -1) {
        // 重建内容
        const afterFunction = lines.slice(functionEndIndex).join('\n');
        content = beforeMarker + '\n' + afterFunction;
      } else {
        console.log('⚠️  无法确定函数结束位置');
        return false;
      }
    }

    // 方法2: 如果没找到标记，使用正则表达式清理任何 fnva 相关内容
    if (content === originalContent) {
      // 使用正则表达式删除任何包含 fnva 的行和相关的环境变量处理
      content = content
        // 删除标记到函数结束的所有内容
        .replace(/# fnva 自动化函数 - 由 npm 安装自动添加[\s\S]*?(?=\n\S|\n$)/g, '')
        // 删除剩余的 fnva 相关行
        .replace(/.*fnva.*\n?/g, '')
        // 删除 FNVAAUTOMODE 相关行
        .replace(/.*FNVAAUTOMODE.*\n?/g, '')
        // 删除 cmd.exe 调用 fnva 的行
        .replace(/.*cmd\.exe.*fnva.*\n?/g, '')
        // 清理多余的空行
        .replace(/\n{3,}/g, '\n\n')
        .trim() + '\n';
    }

    // 如果内容有变化，写入文件
    if (content !== originalContent) {
      fs.writeFileSync(configPath, content);
      console.log(`✅ fnva shell 集成已从 ${configPath} 移除`);
      return true;
    } else {
      console.log('⚠️  未找到需要清理的内容');
      return false;
    }
  } catch (error) {
    console.log(`❌ 移除失败: ${error.message}`);
    return false;
  }
}

function main() {
  console.log('🔧 fnva shell 集成卸载器');

  const shell = detectShell();
  const configPath = getShellConfigPath(shell);

  if (!configPath) {
    console.log(`❌ 不支持的 shell: ${shell}`);
    return;
  }

  const success = removeShellIntegration(configPath, shell);

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
  getShellConfigPath,
  removeShellIntegration
};
#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

/**
 * 检查platforms目录中二进制文件的权限
 */
function checkPermissions() {
  console.log('🔍 检查二进制文件权限...');

  const platformsDir = path.join(__dirname, '..', 'platforms');

  if (!fs.existsSync(platformsDir)) {
    console.log('❌ platforms目录不存在');
    process.exit(1);
  }

  const platforms = fs.readdirSync(platformsDir);
  let allGood = true;

  for (const platform of platforms) {
    const platformDir = path.join(platformsDir, platform);

    if (!fs.statSync(platformDir).isDirectory()) continue;

    const binaryName = platform.includes('win32') ? 'fnva.exe' : 'fnva';
    const binaryPath = path.join(platformDir, binaryName);

    if (fs.existsSync(binaryPath)) {
      const stats = fs.statSync(binaryPath);
      const hasExecPermission = (stats.mode & 0o111) !== 0;
      const mode = stats.mode.toString(8).padStart(4, '0');

      console.log(`   ${platform}/${binaryName}: ${mode} ${hasExecPermission ? '✅' : '❌'}`);

      if (!hasExecPermission && binaryName !== 'fnva.exe') {
        allGood = false;
      }
    } else {
      console.log(`   ${platform}/${binaryName}: ❌ 文件不存在`);
      allGood = false;
    }
  }

  console.log(`\n${allGood ? '✅' : '❌'} 权限检查${allGood ? '通过' : '失败'}`);

  if (!allGood) {
    console.log('\n修复建议:');
    console.log('  运行以下命令设置权限:');
    console.log('  find platforms -name "fnva" -type f -exec chmod 755 {} \\;');
  }
}

if (require.main === module) {
  checkPermissions();
}

module.exports = { checkPermissions };
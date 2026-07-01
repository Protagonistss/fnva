#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

/**
 * 检查 platforms 目录中二进制文件的权限
 */
function checkPermissions() {
  console.log('🔍 Checking binary file permissions...');

  const platformsDir = path.join(__dirname, '..', 'platforms');

  if (!fs.existsSync(platformsDir)) {
    console.log('❌ platforms directory does not exist');
    process.exit(1);
  }

  const platforms = fs.readdirSync(platformsDir);
  let allGood = true;

  // 优先检查新的 platform-arch 目录结构
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
      console.log(`   ${platform}/${binaryName}: ❌ File does not exist`);
      allGood = false;
    }
  }

  // 额外检查一次扁平结构: platforms/fnva
  const flatBinaryName = process.platform === 'win32' ? 'fnva.exe' : 'fnva';
  const flatBinaryPath = path.join(platformsDir, flatBinaryName);

  if (fs.existsSync(flatBinaryPath)) {
    const stats = fs.statSync(flatBinaryPath);
    const hasExecPermission = (stats.mode & 0o111) !== 0;
    const mode = stats.mode.toString(8).padStart(4, '0');

    console.log(`   (legacy)/${flatBinaryName}: ${mode} ${hasExecPermission ? '✅' : '❌'}`);

    if (!hasExecPermission && flatBinaryName !== 'fnva.exe') {
      allGood = false;
    }
  }

  console.log(`\n${allGood ? '✅' : '❌'} Permission check ${allGood ? 'passed' : 'failed'}`);

  if (!allGood) {
    console.log('\nSuggested fix:');
    console.log('  Run the following command to set permissions:');
    console.log('  find platforms -name "fnva" -type f -exec chmod 755 {} \\;');
  }
}

if (require.main === module) {
  checkPermissions();
}

module.exports = { checkPermissions };


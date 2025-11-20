#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

/**
 * 确保fnva二进制文件有可执行权限
 * 这是一个全面的postinstall脚本，处理本地安装和全局安装的权限问题
 */
function ensureExecutablePermissions() {
  try {
    const scriptDir = __dirname;
    const projectRoot = path.resolve(scriptDir, '..');
    const platformsDir = path.join(projectRoot, 'platforms');

    console.log('🔧 Ensuring fnva binary permissions...');

    // 如果没有platforms目录，说明是开发模式，不需要处理
    if (!fs.existsSync(platformsDir)) {
      console.log('ℹ️  No platforms directory found, skipping permission check');
      return;
    }

    // 检测当前平台
    const platform = process.platform;
    const arch = process.arch === 'arm64' ? 'arm64' : 'x64';
    const platformDir = `${platform}-${arch}`;

    // 确定二进制文件名和路径
    const binaryName = platform === 'win32' ? 'fnva.exe' : 'fnva';
    const binaryPath = path.join(platformsDir, platformDir, binaryName);

    // 如果二进制文件存在且不是Windows，设置可执行权限
    if (fs.existsSync(binaryPath) && platform !== 'win32') {
      try {
        const stats = fs.statSync(binaryPath);
        const hasExecPermission = (stats.mode & 0o111) !== 0;

        console.log(`📍 Checking binary: ${binaryPath}`);
        console.log(`   Current permissions: ${(stats.mode & 0o777).toString(8)}`);

        if (!hasExecPermission) {
          console.log(`🔧 Setting executable permissions...`);
          fs.chmodSync(binaryPath, 0o755); // rwxr-xr-x

          // 验证权限设置成功
          const newStats = fs.statSync(binaryPath);
          const newHasExecPermission = (newStats.mode & 0o111) !== 0;

          if (newHasExecPermission) {
            console.log(`✅ Successfully set executable permissions (${platformDir})`);
          } else {
            console.log(`❌ Failed to set executable permissions (${platformDir})`);
            console.log(`   New permissions: ${(newStats.mode & 0o777).toString(8)}`);
            console.log(`   Manual fix may be required: chmod +x "${binaryPath}"`);
          }
        } else {
          console.log(`✅ fnva binary already has executable permissions (${platformDir})`);
        }

        // 尝试测试二进制文件是否可以执行（简单测试）
        try {
          const { spawnSync } = require('child_process');
          const testResult = spawnSync(binaryPath, ['--version'], {
            encoding: 'utf8',
            timeout: 3000,
            stdio: 'pipe'
          });

          if (testResult.status === 0 || testResult.status === 1) { // status 1 可能是正常的错误状态
            console.log(`✅ fnva binary is executable and responding`);
          } else if (testResult.error && testResult.error.code === 'EACCES') {
            console.log(`❌ fnva binary still has permission issues`);
            console.log(`   Manual fix required: chmod +x "${binaryPath}"`);
          }
        } catch (testError) {
          // 测试失败不算严重错误，可能是因为二进制文件本身有问题
        }

      } catch (error) {
        console.warn(`⚠️  Could not fix binary permissions: ${error.message}`);
        console.log(`   Manual fix required: chmod +x "${binaryPath}"`);
      }
    } else if (platform === 'win32') {
      console.log(`ℹ️  Windows platform detected, skipping permission check`);
    } else {
      console.log(`❌ Binary not found: ${binaryPath}`);
      console.log(`   This might indicate an incomplete installation`);
    }

    // 额外检查：如果是全局安装，也检查全局路径中的fnva
    if (process.env.npm_config_global === 'true') {
      try {
        const { execSync } = require('child_process');
        const globalFnvaPath = execSync('which fnva', { encoding: 'utf8' }).trim();

        if (globalFnvaPath && fs.existsSync(globalFnvaPath)) {
          console.log(`📍 Checking globally installed binary: ${globalFnvaPath}`);

          const globalStats = fs.statSync(globalFnvaPath);
          const globalHasExecPermission = (globalStats.mode & 0o111) !== 0;

          if (!globalHasExecPermission) {
            console.log(`🔧 Global fnva binary lacks executable permissions`);
            console.log(`   Please run: sudo chmod +x "${globalFnvaPath}"`);
          } else {
            console.log(`✅ Global fnva binary has correct permissions`);
          }
        }
      } catch (globalError) {
        // 无法检查全局安装，不视为错误
        console.log(`ℹ️  Could not verify global installation`);
      }
    }

  } catch (error) {
    console.warn(`⚠️  Permission check failed: ${error.message}`);
  }
}

// 运行权限检查
ensureExecutablePermissions();
#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

/**
 * 确保 fnva 二进制文件有可执行权限
 * 这是一个全局的 postinstall 脚本，处理本地安装和全局安装的权限问题
 */
function ensureExecutablePermissions() {
  try {
    const scriptDir = __dirname;
    const projectRoot = path.resolve(scriptDir, '..');
    const platformsDir = path.join(projectRoot, 'platforms');

    console.log('✅ Ensuring fnva binary permissions...');

    // 如果没有 platforms 目录，说明是开发模式，不需要处理
    if (!fs.existsSync(platformsDir)) {
      console.log('ℹ️  No platforms directory found, skipping permission check');
      return;
    }

    const platform = process.platform;
    const arch = process.arch === 'arm64' ? 'arm64' : 'x64';
    const platformDir = `${platform}-${arch}`;

    const binaryName = platform === 'win32' ? 'fnva.exe' : 'fnva';
    const archBinaryPath = path.join(platformsDir, platformDir, binaryName);
    const flatBinaryPath = path.join(platformsDir, binaryName);

    /**
     * 确保指定路径的二进制文件具有可执行权限，并做一次简单的运行测试
     */
    function ensureExecutable(binaryPath, label) {
      try {
        const stats = fs.statSync(binaryPath);
        const hasExecPermission = (stats.mode & 0o111) !== 0;

        console.log(`📍 Checking binary (${label}): ${binaryPath}`);
        console.log(`   Current permissions: ${(stats.mode & 0o777).toString(8)}`);

        if (!hasExecPermission) {
          console.log('🔧 Setting executable permissions...');
          fs.chmodSync(binaryPath, 0o755); // rwxr-xr-x

          const newStats = fs.statSync(binaryPath);
          const newHasExecPermission = (newStats.mode & 0o111) !== 0;

          if (newHasExecPermission) {
            console.log(`✅ Successfully set executable permissions (${label})`);
          } else {
            console.log(`❌ Failed to set executable permissions (${label})`);
            console.log(`   New permissions: ${(newStats.mode & 0o777).toString(8)}`);
            console.log(`   Manual fix may be required: chmod +x "${binaryPath}"`);
          }
        } else {
          console.log(`✅ fnva binary already has executable permissions (${label})`);
        }

        // 尝试执行一次 --version 做简单验证
        try {
          const { spawnSync } = require('child_process');
          const testResult = spawnSync(binaryPath, ['--version'], {
            encoding: 'utf8',
            timeout: 3000,
            stdio: 'pipe',
          });

          if (testResult.status === 0 || testResult.status === 1) {
            console.log('✅ fnva binary is executable and responding');
          } else if (testResult.error && testResult.error.code === 'EACCES') {
            console.log('❌ fnva binary still has permission issues');
            console.log(`   Manual fix required: chmod +x "${binaryPath}"`);
          }
        } catch {
          // 测试失败不视为致命错误，可能是二进制本身的问题
        }
      } catch (error) {
        console.warn(`⚠️  Could not fix binary permissions (${label}): ${error.message}`);
        console.log(`   Manual fix required: chmod +x "${binaryPath}"`);
      }
    }

    // Windows 不需要 chmod，可直接跳过
    if (platform === 'win32') {
      console.log('ℹ️  Windows platform detected, skipping permission check');
    } else if (fs.existsSync(archBinaryPath)) {
      // 优先处理新的平台子目录结构: platforms/<platform>-<arch>/fnva
      ensureExecutable(archBinaryPath, platformDir);
    } else if (fs.existsSync(flatBinaryPath)) {
      // 兼容旧版本扁平结构: platforms/fnva
      console.log('ℹ️  Platform-specific binary not found, falling back to legacy flat layout');
      ensureExecutable(flatBinaryPath, 'platforms/fnva');
    } else {
      console.log(`❌ Binary not found: ${archBinaryPath}`);
      console.log(`   Also checked legacy path: ${flatBinaryPath}`);
      console.log('   This might indicate an incomplete installation');
    }

    // 额外检查：如果是全局安装，也尝试检查路径上的 fnva 权限
    if (process.env.npm_config_global === 'true') {
      try {
        const { execSync } = require('child_process');
        const globalFnvaPath = execSync('which fnva', { encoding: 'utf8' }).trim();

        if (globalFnvaPath && fs.existsSync(globalFnvaPath)) {
          console.log(`📍 Checking globally installed binary: ${globalFnvaPath}`);

          const globalStats = fs.statSync(globalFnvaPath);
          const globalHasExecPermission = (globalStats.mode & 0o111) !== 0;

          if (!globalHasExecPermission) {
            console.log('❌ Global fnva binary lacks executable permissions');
            console.log(`   Please run: sudo chmod +x "${globalFnvaPath}"`);
          } else {
            console.log('✅ Global fnva binary has correct permissions');
          }
        }
      } catch {
        console.log('ℹ️  Could not verify global installation');
      }
    }
  } catch (error) {
    console.warn(`⚠️  Permission check failed: ${error.message}`);
  }
}

// 运行权限检查
ensureExecutablePermissions();


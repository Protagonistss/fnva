#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

/**
 * 确保fnva二进制文件有可执行权限
 * 这是一个轻量级的postinstall脚本，专门用来解决npm打包时权限丢失的问题
 */
function ensureExecutablePermissions() {
  try {
    const scriptDir = __dirname;
    const projectRoot = path.resolve(scriptDir, '..');
    const platformsDir = path.join(projectRoot, 'platforms');

    // 如果没有platforms目录，说明是开发模式，不需要处理
    if (!fs.existsSync(platformsDir)) {
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

        if (!hasExecPermission) {
          fs.chmodSync(binaryPath, 0o755); // rwxr-xr-x
          // 只在实际修复了权限时才输出消息，避免在正常安装时产生噪音
          if (process.env.DEBUG || process.env.NPM_DEBUG) {
            console.log(`🔧 Fixed executable permissions for fnva binary`);
          }
        }
      } catch (error) {
        // 静默处理错误，不干扰正常安装流程
        if (process.env.DEBUG || process.env.NPM_DEBUG) {
          console.warn(`⚠️  Could not fix binary permissions: ${error.message}`);
        }
      }
    }
  } catch (error) {
    // 静默处理错误，不干扰正常安装流程
    if (process.env.DEBUG || process.env.NPM_DEBUG) {
      console.warn(`⚠️  Permission check failed: ${error.message}`);
    }
  }
}

// 运行权限检查
ensureExecutablePermissions();
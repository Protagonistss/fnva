/**
 * 编码处理工具模块
 * 用于统一处理跨平台字符编码问题
 */

const fs = require('fs');
const path = require('path');

/**
 * 编码处理工具类
 */
class EncodingUtils {
  /**
   * 设置Windows控制台编码为UTF-8
   */
  static setWindowsConsoleEncoding() {
    if (process.platform === 'win32') {
      try {
        // 尝试设置控制台编码为UTF-8
        const { execSync } = require('child_process');
        execSync('chcp 65001 > nul 2>&1', { stdio: 'ignore' });

        // 设置Node.js输出编码
        if (process.stdout._handle && process.stdout._handle.setEncoding) {
          process.stdout._handle.setEncoding('utf8');
        }
        if (process.stderr._handle && process.stderr._handle.setEncoding) {
          process.stderr._handle.setEncoding('utf8');
        }
      } catch (error) {
        // 静默忽略错误，避免影响正常功能
      }
    }
  }

  /**
   * 为文件内容添加适当的编码标记
   * @param {string} content - 文件内容
   * @param {string} filePath - 文件路径（用于确定文件类型）
   * @returns {string} - 处理后的内容
   */
  static addEncodingSignature(content, filePath) {
    // 为Windows PowerShell脚本添加BOM
    if (process.platform === 'win32' && this.isPowerShellScript(filePath)) {
      return '\ufeff' + content;
    }
    return content;
  }

  /**
   * 检查文件是否为PowerShell脚本
   * @param {string} filePath - 文件路径
   * @returns {boolean} - 是否为PowerShell脚本
   */
  static isPowerShellScript(filePath) {
    const ext = path.extname(filePath).toLowerCase();
    return ext === '.ps1';
  }

  /**
   * 安全地写入文件，自动处理编码
   * @param {string} filePath - 文件路径
   * @param {string} content - 文件内容
   * @param {string} encoding - 编码格式，默认为utf8
   */
  static writeFileWithEncoding(filePath, content, encoding = 'utf8') {
    const processedContent = this.addEncodingSignature(content, filePath);
    fs.writeFileSync(filePath, processedContent, encoding);
  }

  /**
   * 生成PowerShell编码设置脚本
   * @returns {string} - PowerShell编码设置代码
   */
  static generatePowerShellEncodingSetup() {
    return [
      '# 设置UTF-8编码以正确显示中文',
      '[Console]::OutputEncoding = [System.Text.Encoding]::UTF8',
      '$OutputEncoding = [System.Console]::OutputEncoding',
      ''
    ].join('\n');
  }

  /**
   * 检测系统默认编码
   * @returns {string} - 系统编码名称
   */
  static detectSystemEncoding() {
    if (process.platform === 'win32') {
      try {
        const { execSync } = require('child_process');
        const result = execSync('chcp', { encoding: 'utf8' });
        const match = result.match(/活动代码页: (\d+)/);
        return match ? `cp${match[1]}` : 'utf8';
      } catch (e) {
        return 'cp936'; // Windows中文默认编码
      }
    } else {
      return 'utf8'; // Unix-like系统默认UTF-8
    }
  }

  /**
   * 创建临时PowerShell脚本文件
   * @param {string} content - 脚本内容
   * @param {string} prefix - 文件名前缀
   * @returns {string} - 临时文件路径
   */
  static createTempPowerShellScript(content, prefix = 'fnva') {
    const os = require('os');
    const path = require('path');

    const tempDir = os.tmpdir();
    const timestamp = Date.now();
    const scriptFile = path.join(tempDir, `${prefix}_${timestamp}.ps1`);

    // 在脚本开头添加编码设置
    const encodingSetup = this.generatePowerShellEncodingSetup();
    const fullContent = encodingSetup + content;

    this.writeFileWithEncoding(scriptFile, fullContent);

    return scriptFile;
  }

  /**
   * 安全地执行PowerShell脚本，确保编码正确
   * @param {string} scriptPath - 脚本路径
   * @param {Array} args - 传递给PowerShell的参数
   * @param {Object} options - 执行选项
   * @returns {Object} - 执行结果
   */
  static executePowerShellScript(scriptPath, args = [], options = {}) {
    const { spawn } = require('child_process');

    const defaultOptions = {
      stdio: 'inherit',
      shell: false,
      ...options
    };

    const psArgs = [
      '-ExecutionPolicy', 'Bypass',
      '-File', scriptPath,
      ...args
    ];

    return spawn('powershell', psArgs, defaultOptions);
  }
}

module.exports = EncodingUtils;
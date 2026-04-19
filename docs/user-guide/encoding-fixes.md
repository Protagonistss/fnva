# FNVA 中文字符编码修复文档

## 概述

本文档描述了对FNVA项目中文字符编码乱码问题的修复方案。主要解决Windows PowerShell环境下中文显示乱码的问题。

## 问题背景

FNVA项目是一个跨平台环境切换工具，在Windows PowerShell环境下存在以下编码问题：

1. **PowerShell控制台编码冲突**：Windows默认使用GBK/CP936编码，而Node.js和Rust使用UTF-8编码
2. **临时脚本文件编码问题**：生成的PowerShell脚本文件缺乏正确的编码标识
3. **进程间编码传递问题**：Rust程序输出到Node.js再到PowerShell过程中的编码转换问题

## 修复方案

### 1. PowerShell脚本编码设置

**位置**：
- `bin/fnva.js` - Node.js包装器
- `src/infrastructure/shell/script_strategy.rs` - Rust核心代码

**修复内容**：
在所有生成的PowerShell脚本开头添加编码设置：
```powershell
# 设置UTF-8编码以正确显示中文
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Console]::OutputEncoding
```

### 2. 文件BOM标记处理

**位置**：`lib/encoding-utils.js`

**修复内容**：
创建编码处理工具模块，为Windows环境下的PowerShell文件添加UTF-8 BOM标记：
```javascript
const content = process.platform === 'win32' ? '\ufeff' + script : script;
```

### 3. 控制台编码检测

**位置**：`lib/encoding-utils.js`

**修复内容**：
在Node.js启动时自动设置Windows控制台编码：
```javascript
execSync('chcp 65001 > nul 2>&1', { stdio: 'ignore' });
```

## 修复的文件

### JavaScript/Node.js文件

1. **`bin/fnva.js`**
   - 引入编码工具模块
   - 修复`generateSimpleScript`函数
   - 修复`createTempScriptFile`函数
   - 添加控制台编码设置调用

2. **`lib/encoding-utils.js`**（新增）
   - 编码处理工具类
   - BOM标记添加功能
   - 控制台编码设置
   - 系统编码检测

### Rust文件

1. **`src/infrastructure/shell/script_strategy.rs`**
   - 在PowerShell模板中添加编码设置
   - 修复`POWERSHELL_JAVA_SWITCH_TEMPLATE`
   - 修复`POWERSHELL_LLM_SWITCH_TEMPLATE`

## 使用说明

### 开发者注意事项

1. **新增PowerShell脚本时**：必须在脚本开头包含编码设置代码
2. **文件写入操作**：使用`EncodingUtils.writeFileWithEncoding()`替代直接的`fs.writeFileSync()`
3. **临时脚本生成**：使用`EncodingUtils.createTempPowerShellScript()`确保正确的编码处理

### 用户使用指南

1. **Windows PowerShell用户**：修复后的脚本将自动设置正确的编码，中文显示应该正常
2. **其他系统用户**：修复不影响其他系统的正常使用
3. **故障排除**：如果仍然遇到乱码，请检查PowerShell版本和系统编码设置

## 技术细节

### 编码设置原理

1. **UTF-8 BOM**：帮助Windows正确识别UTF-8编码的文件
2. **PowerShell编码设置**：
   - `[Console]::OutputEncoding`：设置控制台输出编码
   - `$OutputEncoding`：设置PowerShell管道输出编码
3. **控制台编码切换**：使用`chcp 65001`切换到UTF-8代码页

### 兼容性考虑

1. **向后兼容**：修复不影响现有功能的正常使用
2. **跨平台支持**：编码设置仅在Windows平台生效
3. **错误处理**：编码设置失败不会影响主要功能

## 测试建议

### 测试场景

1. **Windows PowerShell环境**：
   ```bash
   fnva java use <version>
   fnva llm use <config>
   ```

2. **PowerShell ISE**：测试在PowerShell ISE中的显示效果
3. **Windows Terminal**：测试在现代终端中的显示效果
4. **其他系统**：确保修复不破坏macOS和Linux的功能

### 验证方法

1. **中文显示检查**：确保所有中文字符正常显示
2. **emoji显示检查**：确保emoji图标正常显示
3. **功能完整性**：确保环境切换功能正常工作

## 后续维护

1. **代码审查**：新增代码需要遵循编码处理规范
2. **测试覆盖**：定期在不同环境中测试编码效果
3. **文档更新**：如需修改，请更新此文档

## 相关资源

- [PowerShell编码文档](https://docs.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_character_encoding)
- [Node.js编码处理](https://nodejs.org/api/buffer.html#buffer_buffers_and_character_encodings)
- [UTF-8 BOM说明](https://en.wikipedia.org/wiki/Byte_order_mark)
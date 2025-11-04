# GitHub Actions 工作流

## build.yml

自动构建和发布工作流，支持以下功能：

### 触发条件

1. **推送标签**：当推送以 `v` 开头的标签时（如 `v0.1.0`）
2. **手动触发**：可以通过 GitHub Actions 界面手动触发

### 构建的平台

- ✅ Linux x64
- ✅ Linux ARM64
- ✅ macOS Intel (x64)
- ✅ macOS Apple Silicon (ARM64)
- ✅ Windows x64
- ✅ Windows ARM64

### 工作流步骤

1. **构建阶段**：为每个平台并行构建二进制文件
2. **准备 Release**：创建 GitHub Release 并上传所有平台的二进制文件
3. **发布到 npm**：自动发布到 npm registry

### 配置要求

#### NPM Token

在 GitHub 仓库设置中添加 `NPM_TOKEN` secret：
1. 前往 GitHub 仓库的 Settings → Secrets and variables → Actions
2. 添加新的 secret，名称为 `NPM_TOKEN`
3. 值来自 npm 账户的 Access Token（需要 publish 权限）

#### 创建 Release

```bash
# 创建并推送标签
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions 会自动：
1. 构建所有平台的二进制文件
2. 创建 GitHub Release
3. 发布到 npm

### 本地测试

在发布前，可以本地测试构建：

```bash
# 构建当前平台
npm run build

# 测试本地安装
npm pack
npm install -g nva-*.tgz
```


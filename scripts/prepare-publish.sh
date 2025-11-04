#!/bin/bash

# 发布前准备脚本

set -e

echo "准备发布 npm 包..."

# 检查必要的文件
REQUIRED_FILES=(
    "package.json"
    "bin/fnva"
    "README.md"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [ ! -f "$file" ]; then
        echo "错误: 缺少必需文件: $file"
        exit 1
    fi
done

# 检查版本号
echo "检查版本号..."
VERSION=$(node -p "require('./package.json').version")
echo "当前版本: $VERSION"

# 在 CI/CD 环境中跳过平台二进制文件检查和交互式确认
if [ -n "$CI" ] || [ -n "$GITHUB_ACTIONS" ]; then
    echo "CI/CD 环境: 跳过构建检查，自动确认发布版本 $VERSION"
    echo "平台二进制文件将在发布时包含"
else
    # 本地环境: 检查平台二进制文件
    if [ ! -d "platforms" ] || [ -z "$(ls -A platforms 2>/dev/null)" ]; then
        echo "警告: platforms 目录为空或不存在"
        echo "请先运行 'npm run build' 或 'npm run build:all' 构建二进制文件"
        exit 1
    fi

    # 本地环境: 提示用户确认
    read -p "确认发布版本 $VERSION? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "已取消发布"
        exit 1
    fi
fi

echo "准备完成，准备发布到 NPM"


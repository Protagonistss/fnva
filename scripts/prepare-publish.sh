#!/bin/bash

# 发布前准备脚本

set -e

echo "准备发布 npm 包..."

# 检查必要的文件
REQUIRED_FILES=(
    "package.json"
    "bin/nva"
    "README.md"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [ ! -f "$file" ]; then
        echo "错误: 缺少必需文件: $file"
        exit 1
    fi
done

# 检查是否有平台二进制文件
if [ ! -d "platforms" ] || [ -z "$(ls -A platforms 2>/dev/null)" ]; then
    echo "警告: platforms 目录为空，运行构建脚本..."
    npm run build || {
        echo "错误: 构建失败，请先运行 'npm run build'"
        exit 1
    }
fi

# 检查版本号是否已更新（简单检查）
echo "检查版本号..."
VERSION=$(node -p "require('./package.json').version")
echo "当前版本: $VERSION"

# 提示用户确认
read -p "确认发布版本 $VERSION? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "已取消发布"
    exit 1
fi

echo "准备完成，可以运行 'npm publish' 发布"


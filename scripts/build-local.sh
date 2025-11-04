#!/bin/bash

# 本地构建脚本 - 仅构建当前平台的二进制文件

set -e

echo "开始构建当前平台的二进制文件..."

# 获取项目根目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PLATFORMS_DIR="$PROJECT_ROOT/platforms"

# 创建 platforms 目录
mkdir -p "$PLATFORMS_DIR"

# 检测当前平台
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# 确定目标平台
case "$OS" in
  darwin)
    if [ "$ARCH" = "arm64" ] || [ "$ARCH" = "aarch64" ]; then
      TARGET="aarch64-apple-darwin"
      PLATFORM="darwin-arm64"
    else
      TARGET="x86_64-apple-darwin"
      PLATFORM="darwin-x64"
    fi
    BINARY_NAME="nva"
    ;;
  linux)
    if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
      TARGET="aarch64-unknown-linux-gnu"
      PLATFORM="linux-arm64"
    else
      TARGET="x86_64-unknown-linux-gnu"
      PLATFORM="linux-x64"
    fi
    BINARY_NAME="nva"
    ;;
  *)
    echo "错误: 不支持的操作系统: $OS"
    echo "请使用 GitHub Actions 构建其他平台"
    exit 1
    ;;
esac

echo "检测到平台: $OS ($ARCH)"
echo "目标平台: $TARGET"
echo "平台目录: $PLATFORM"

# 构建
echo ""
echo "开始构建..."
cargo build --release --target "$TARGET"

# 准备输出目录
OUTPUT_DIR="$PLATFORMS_DIR/$PLATFORM"
mkdir -p "$OUTPUT_DIR"

# 复制二进制文件
SOURCE_BINARY="$PROJECT_ROOT/target/$TARGET/release/$BINARY_NAME"
if [ -f "$SOURCE_BINARY" ]; then
  cp "$SOURCE_BINARY" "$OUTPUT_DIR/$BINARY_NAME"
  echo "✓ 成功构建: $OUTPUT_DIR/$BINARY_NAME"
  
  # 优化二进制文件大小
  if command -v strip &> /dev/null; then
    strip "$OUTPUT_DIR/$BINARY_NAME"
    echo "✓ 已优化二进制文件大小"
  fi
  
  # 显示文件大小
  ls -lh "$OUTPUT_DIR/$BINARY_NAME"
else
  echo "✗ 错误: 未找到构建产物: $SOURCE_BINARY"
  exit 1
fi

echo ""
echo "=========================================="
echo "构建完成！"
echo "二进制文件位置: $OUTPUT_DIR/$BINARY_NAME"
echo "=========================================="
echo ""
echo "注意: 此脚本仅构建当前平台。"
echo "要构建所有平台，请使用 GitHub Actions 或运行 scripts/build-all.sh"


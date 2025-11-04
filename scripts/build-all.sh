#!/bin/bash

# 构建所有平台的二进制文件

set -e

echo "开始构建所有平台的二进制文件..."

# 获取项目根目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PLATFORMS_DIR="$PROJECT_ROOT/platforms"

# 创建 platforms 目录
mkdir -p "$PLATFORMS_DIR"

# 检查是否安装了 cross（用于交叉编译）
if ! command -v cross &> /dev/null; then
    echo "警告: 未找到 cross 工具，尝试安装..."
    cargo install cross --git https://github.com/cross-rs/cross
fi

# 定义目标平台
TARGETS=(
    "x86_64-apple-darwin"      # macOS Intel
    "aarch64-apple-darwin"     # macOS Apple Silicon
    "x86_64-unknown-linux-gnu" # Linux x64
    "aarch64-unknown-linux-gnu" # Linux ARM64
    "x86_64-pc-windows-msvc"   # Windows x64
    "aarch64-pc-windows-msvc"  # Windows ARM64
)

# 构建函数
build_target() {
    local target=$1
    local platform_name=$2
    
    echo ""
    echo "=========================================="
    echo "构建目标: $target"
    echo "平台名称: $platform_name"
    echo "=========================================="
    
    # 使用 cross 交叉编译
    if command -v cross &> /dev/null; then
        cross build --release --target "$target"
    else
        # 如果没有 cross，尝试直接使用 cargo（仅适用于当前平台）
        cargo build --release --target "$target"
    fi
    
    # 确定输出目录和文件名
    local output_dir="$PLATFORMS_DIR/$platform_name"
    mkdir -p "$output_dir"
    
    # 确定二进制文件名
    if [[ "$target" == *"windows"* ]]; then
        local binary_name="nva.exe"
    else
        local binary_name="nva"
    fi
    
    # 复制二进制文件
    local source_binary="$PROJECT_ROOT/target/$target/release/$binary_name"
    
    if [ -f "$source_binary" ]; then
        cp "$source_binary" "$output_dir/$binary_name"
        echo "✓ 成功构建: $output_dir/$binary_name"
        
        # 可选：压缩二进制文件（使用 strip）
        if command -v strip &> /dev/null && [[ "$binary_name" != "*.exe" ]]; then
            strip "$output_dir/$binary_name"
            echo "✓ 已优化二进制文件大小"
        fi
    else
        echo "✗ 错误: 未找到构建产物: $source_binary"
        return 1
    fi
}

# 构建所有目标
for target_info in "${TARGETS[@]}"; do
    IFS='|' read -r target platform <<< "$target_info"
    if [ -z "$platform" ]; then
        # 如果没有指定平台名称，从 target 中提取
        platform=$(echo "$target" | sed 's/.*-//' | sed 's/gnu$/linux/' | sed 's/msvc$/win32/')
        if [[ "$target" == *"apple-darwin"* ]]; then
            if [[ "$target" == *"aarch64"* ]]; then
                platform="darwin-arm64"
            else
                platform="darwin-x64"
            fi
        elif [[ "$target" == *"linux"* ]]; then
            if [[ "$target" == *"aarch64"* ]]; then
                platform="linux-arm64"
            else
                platform="linux-x64"
            fi
        elif [[ "$target" == *"windows"* ]]; then
            if [[ "$target" == *"aarch64"* ]]; then
                platform="win32-arm64"
            else
                platform="win32-x64"
            fi
        fi
    fi
    
    # 简化平台名称映射
    case "$target" in
        "x86_64-apple-darwin")
            platform="darwin-x64"
            ;;
        "aarch64-apple-darwin")
            platform="darwin-arm64"
            ;;
        "x86_64-unknown-linux-gnu")
            platform="linux-x64"
            ;;
        "aarch64-unknown-linux-gnu")
            platform="linux-arm64"
            ;;
        "x86_64-pc-windows-msvc")
            platform="win32-x64"
            ;;
        "aarch64-pc-windows-msvc")
            platform="win32-arm64"
            ;;
    esac
    
    build_target "$target" "$platform" || echo "跳过 $target（构建失败或平台不支持）"
done

echo ""
echo "=========================================="
echo "构建完成！"
echo "二进制文件位置: $PLATFORMS_DIR"
echo "=========================================="


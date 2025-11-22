#!/bin/bash

# 代码风格检查脚本
# 用于检查和格式化fnva项目的代码风格

set -e

echo "🔍 开始代码风格检查..."

# 检查是否安装了必要的工具
check_tool() {
    if ! command -v $1 &> /dev/null; then
        echo "❌ $1 未安装，请先安装 $1"
        echo "安装命令: cargo install $1"
        exit 1
    fi
}

echo "📦 检查工具是否安装..."
check_tool "rustfmt"
check_tool "clippy"

# 运行rustfmt格式化代码
echo "🎨 格式化代码..."
cargo fmt --all

# 运行clippy检查
echo "🔍 运行Clippy静态分析..."
cargo clippy --all-targets --all-features -- -D warnings

# 检查是否有未提交的格式化更改
echo "📝 检查格式化结果..."
if ! git diff --exit-code --quiet; then
    echo "⚠️  代码格式化产生了更改，请提交这些更改"
    echo "运行 'git add .' 和 'git commit' 来提交格式化结果"
    exit 1
fi

# 检查文档注释
echo "📚 检查文档注释..."
cargo doc --no-deps --document-private-items 2>/dev/null | grep -E "(warning|error)" || true

# 检查重复的代码
echo "🔄 检查重复代码..."
if command -v cargo-dup &> /dev/null; then
    cargo dup
else
    echo "💡 提示: 安装 cargo-dup 可以检查重复代码 (cargo install cargo-dup)"
fi

# 检查依赖安全性
echo "🔒 检查依赖安全性..."
if command -v cargo-audit &> /dev/null; then
    cargo audit
else
    echo "💡 提示: 安装 cargo-audit 可以检查依赖安全性 (cargo install cargo-audit)"
fi

# 检查未使用的依赖
echo "🧹 检查未使用的依赖..."
if command -v cargo-udeps &> /dev/null; then
    cargo udeps --all-targets
else
    echo "💡 提示: 安装 cargo-udeps 可以检查未使用的依赖 (cargo install cargo-udeps)"
fi

# 统计代码行数
echo "📊 代码统计:"
echo "总Rust代码行数: $(find src -name '*.rs' -exec wc -l {} + | tail -1)"
echo "测试代码行数: $(find tests -name '*.rs' -exec wc -l {} + 2>/dev/null | tail -1 || echo "0")"

echo "✅ 代码风格检查完成！"
#!/bin/bash

# 代码格式化脚本
# 自动格式化fnva项目的代码

set -e

echo "🎨 开始格式化代码..."

# 格式化所有Rust代码
cargo fmt --all

# 检查格式化结果
if git diff --exit-code --quiet; then
    echo "✅ 代码已经是标准格式"
else
    echo "📝 代码已格式化，请查看更改:"
    git diff --stat
    echo ""
    echo "💡 提示: 使用 'git add .' 和 'git commit' 提交格式化更改"
fi

echo "✅ 格式化完成！"
#!/bin/bash

# fnva权限修复脚本 - 临时解决方案
# 适用于Mac/Linux系统的全局安装权限修复

set -e

echo "🔧 fnva权限修复工具"
echo "=================="

# 检查fnva是否已安装
if ! command -v fnva &> /dev/null; then
    echo "❌ fnva未找到，请先安装: npm install -g fnva"
    exit 1
fi

# 获取fnva路径
FNVA_PATH=$(which fnva)
echo "📍 找到fnva: $FNVA_PATH"

# 检查权限
if [ -x "$FNVA_PATH" ]; then
    echo "✅ fnva已有可执行权限"
    echo "🧪 测试fnva是否正常工作..."
    if fnva --version &> /dev/null || [ $? -eq 1 ]; then
        echo "✅ fnva正常工作！"
        exit 0
    else
        echo "⚠️  fnva有权限但执行仍有问题"
    fi
else
    echo "❌ fnva缺少可执行权限"
fi

# 尝试修复权限
echo "🔧 修复fnva权限..."
if sudo chmod +x "$FNVA_PATH"; then
    echo "✅ 权限修复成功"

    # 验证修复结果
    echo "🧪 验证fnva是否正常工作..."
    if fnva --version &> /dev/null || [ $? -eq 1 ]; then
        echo "🎉 fnva权限修复完成！现在可以使用fnva了"
        exit 0
    else
        echo "⚠️  权限已修复但执行仍有问题"
    fi
else
    echo "❌ 权限修复失败"
fi

# 如果上述方法失败，尝试其他方法
echo ""
echo "🔄 尝试其他修复方法..."

# 方法1: 查找所有fnva二进制文件
echo "🔍 查找所有fnva二进制文件..."
FNVA_FILES=$(find /usr/local /opt /home -name "fnva" -type f 2>/dev/null || true)

if [ -n "$FNVA_FILES" ]; then
    echo "📁 找到以下fnva文件:"
    echo "$FNVA_FILES"
    echo ""

    echo "🔧 修复所有fnva文件的权限..."
    echo "$FNVA_FILES" | while read -r file; do
        if [ -f "$file" ]; then
            echo "  修复: $file"
            sudo chmod +x "$file"
        fi
    done
else
    echo "📁 未找到其他fnva文件"
fi

# 方法2: 查找npm全局目录中的fnva
echo ""
echo "🔍 检查npm全局安装目录..."
NPM_GLOBAL_ROOT=$(npm root -g 2>/dev/null || echo "")
if [ -n "$NPM_GLOBAL_ROOT" ]; then
    FNVA_MODULE_PATH="$NPM_GLOBAL_ROOT/fnva"
    if [ -d "$FNVA_MODULE_PATH" ]; then
        echo "📁 找到fnva模块: $FNVA_MODULE_PATH"
        echo "🔧 修复模块中的二进制文件权限..."

        # 查找模块中的所有fnva文件
        find "$FNVA_MODULE_PATH" -name "fnva" -type f -exec sudo chmod +x {} \; 2>/dev/null || true
    fi
fi

# 最终测试
echo ""
echo "🧪 最终测试..."
if fnva --version &> /dev/null || [ $? -eq 1 ]; then
    echo "🎉 修复成功！fnva现在可以正常使用"
    exit 0
else
    echo "❌ 修复失败，请尝试以下方法:"
    echo "  1. 重新安装: npm uninstall -g fnva && npm install -g fnva --force"
    echo "  2. 手动找到fnva文件并修复权限"
    echo "  3. 使用 FNVA_AUTO_MODE=1 fnva list 使用Node.js模式"
    exit 1
fi
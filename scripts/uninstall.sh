#!/bin/sh
# fnva 卸载脚本(Unix):删 binary + 清 shell rc 的 fnva 块。
#   curl -fSsL https://raw.githubusercontent.com/Protagonistss/fnva/main/scripts/uninstall.sh | sh
# 默认保留 ~/.fnva(配置 + 已装的工具);如需彻底删除,末尾有提示。

set -eu

INSTALL_DIR="${FNVA_INSTALL_DIR:-${HOME}/.fnva/bin}"

# 1. 清 shell rc 的 fnva 块(>>> fnva >>> 到 <<< fnva <<<,含标记)
strip_fnva_block() {
    rc="$1"
    [ -f "$rc" ] || return 0
    if grep -q ">>> fnva >>>" "$rc" 2>/dev/null; then
        awk '/>>> fnva >>>/{skip=1; next} /<<< fnva <<</{skip=0; next} !skip' "$rc" > "${rc}.tmp" && mv "${rc}.tmp" "$rc"
        echo "已从 ${rc} 清除 fnva 块"
    fi
}

SHELL_NAME=$(basename "${SHELL:-sh}")
case "$SHELL_NAME" in
    fish)
        strip_fnva_block "${HOME}/.config/fish/config.fish"
        ;;
    *)
        for rc in "${HOME}/.zshrc" "${HOME}/.bashrc" "${HOME}/.profile"; do
            strip_fnva_block "$rc"
        done
        ;;
esac

# 2. 删 binary
if [ -f "${INSTALL_DIR}/fnva" ]; then
    rm -f "${INSTALL_DIR}/fnva"
    echo "已删除 ${INSTALL_DIR}/fnva"
else
    echo "${INSTALL_DIR}/fnva 不存在(可能已删)"
fi

# 3. 提示(默认保留 ~/.fnva 配置)
echo ""
echo "✓ fnva 已卸载"
echo "  配置目录 ${HOME}/.fnva 已保留(含 config.toml 和已装工具);"
echo "  如需彻底删除:rm -rf \"${HOME}/.fnva\""
echo ""
echo "重开终端使 PATH / 环境变量变更生效。"

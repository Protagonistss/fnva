#!/bin/sh
# fnva 一键安装脚本(Unix)。
#   curl -fSsL https://raw.githubusercontent.com/Protagonistss/fnva/main/scripts/install.sh | sh
#   或: curl -fSsL ... -o install.sh && sh install.sh [--install-dir <dir>]
#
# 行为:检测平台 → 从 GitHub Release latest 下载对应 zip → 解压到
# $FNVA_INSTALL_DIR(默认 ~/.fnva/bin)→ 提示 PATH / shell 集成。

set -eu

REPO="Protagonistss/fnva"
URL_BASE="https://github.com/${REPO}/releases/latest/download"

# 解析 --install-dir
INSTALL_DIR="${FNVA_INSTALL_DIR:-${HOME}/.fnva/bin}"
while [ $# -gt 0 ]; do
    case "$1" in
        --install-dir) INSTALL_DIR="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 1 ;;
    esac
done

# 1. 平台检测
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$OS" in
    linux|darwin) ;;
    *) echo "error: unsupported OS: $OS (fnva 提供 linux/darwin 二进制)" >&2; exit 1 ;;
esac
case "$ARCH" in
    x86_64|amd64) ARCH_TAG="x64" ;;
    aarch64|arm64) ARCH_TAG="arm64" ;;
    *) echo "error: unsupported arch: $ARCH (fnva 提供 x64/arm64)" >&2; exit 1 ;;
esac
PLATFORM="${OS}-${ARCH_TAG}"

# 2. 依赖检查(curl 必须,unzip 或 python3 其一)
command -v curl >/dev/null 2>&1 || { echo "error: curl is required" >&2; exit 1; }

# 3. 下载 + 解压
mkdir -p "$INSTALL_DIR"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo "Downloading fnva (${PLATFORM}) from GitHub Release..."
if ! curl -fSsL "${URL_BASE}/${PLATFORM}.zip" -o "${TMP}/fnva.zip"; then
    echo "error: download failed for ${PLATFORM}.zip —— 该平台可能尚未发布,或网络问题" >&2
    exit 1
fi

if command -v unzip >/dev/null 2>&1; then
    unzip -o "${TMP}/fnva.zip" -d "${TMP}" >/dev/null
elif command -v python3 >/dev/null 2>&1; then
    python3 -c "import zipfile,sys; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])" "${TMP}/fnva.zip" "${TMP}"
else
    echo "error: 需要 unzip 或 python3 来解压" >&2; exit 1
fi

mv "${TMP}/fnva" "${INSTALL_DIR}/fnva"
chmod +x "${INSTALL_DIR}/fnva"

# 4. 提示
echo ""
echo "✓ fnva 已安装到 ${INSTALL_DIR}/fnva"
echo ""

case ":${PATH}:" in
    *":${INSTALL_DIR}:"*)
        echo "fnva bin 已在 PATH"
        ;;
    *)
        # 自动加 PATH + shell 集成到对应 rc(检测 shell 类型),带 # fnva 标记便于卸载
        SHELL_NAME=$(basename "${SHELL:-sh}")
        case "$SHELL_NAME" in
            fish)
                RC="${HOME}/.config/fish/config.fish"
                mkdir -p "$(dirname "$RC")"
                if ! grep -q "# fnva" "$RC" 2>/dev/null; then
                    printf '\n# fnva\nset -x PATH %s $PATH\nfnva env | source\n' "$INSTALL_DIR" >> "$RC"
                    echo "已把 PATH + shell 集成加到 ${RC}"
                else
                    echo "${RC} 已有 fnva 配置(跳过)"
                fi
                ;;
            *)
                ADDED=0
                for rc in "${HOME}/.zshrc" "${HOME}/.bashrc" "${HOME}/.profile"; do
                    if [ -f "$rc" ] && ! grep -q "# fnva" "$rc" 2>/dev/null; then
                        printf '\n# fnva\nexport PATH="%s:$PATH"\neval "$(fnva env)"\n' "$INSTALL_DIR" >> "$rc"
                        echo "已把 PATH + shell 集成加到 ${rc}"
                        ADDED=1
                        break
                    fi
                done
                if [ "$ADDED" -eq 0 ]; then
                    echo "未找到 shell rc,请手动加以下两行到配置(~/.bashrc / ~/.zshrc):"
                    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
                    echo "  eval \"\$(fnva env)\""
                fi
                ;;
        esac
        ;;
esac

echo ""
echo "重开终端(或 source rc)后 fnva 完全可用 —— 验证:fnva --version / fnva java use <name>"
echo "卸载:fnva 卸载流程会按 # fnva 标记清理 rc"

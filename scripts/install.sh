#!/bin/sh
# fnva installer (Unix).
#   curl -fSsL https://raw.githubusercontent.com/Protagonistss/fnva/main/scripts/install.sh | sh
#   or: curl -fSsL ... -o install.sh && sh install.sh [--install-dir <dir>]
#
# Downloads the platform binary from GitHub Release latest, extracts to
# $FNVA_INSTALL_DIR (default ~/.fnva/bin), and appends a PATH + shell
# integration block to your shell rc.

set -eu

REPO="Protagonistss/fnva"
URL_BASE="https://github.com/${REPO}/releases/latest/download"

INSTALL_DIR="${FNVA_INSTALL_DIR:-${HOME}/.fnva/bin}"
while [ $# -gt 0 ]; do
    case "$1" in
        --install-dir) INSTALL_DIR="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 1 ;;
    esac
done

# 1. detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$OS" in
    linux|darwin) ;;
    *) echo "error: unsupported OS: $OS (fnva ships linux/darwin binaries)" >&2; exit 1 ;;
esac
case "$ARCH" in
    x86_64|amd64) ARCH_TAG="x64" ;;
    aarch64|arm64) ARCH_TAG="arm64" ;;
    *) echo "error: unsupported arch: $ARCH (fnva ships x64/arm64)" >&2; exit 1 ;;
esac
PLATFORM="${OS}-${ARCH_TAG}"

# 2. deps
command -v curl >/dev/null 2>&1 || { echo "error: curl is required" >&2; exit 1; }

# 3. download + extract
mkdir -p "$INSTALL_DIR"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo "Downloading fnva (${PLATFORM}) from GitHub Release..."
if ! curl -fSsL "${URL_BASE}/${PLATFORM}.zip" -o "${TMP}/fnva.zip"; then
    echo "error: download failed for ${PLATFORM}.zip (platform may not be released yet, or a network issue)" >&2
    exit 1
fi

if command -v unzip >/dev/null 2>&1; then
    unzip -o "${TMP}/fnva.zip" -d "${TMP}" >/dev/null
elif command -v python3 >/dev/null 2>&1; then
    python3 -c "import zipfile,sys; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])" "${TMP}/fnva.zip" "${TMP}"
else
    echo "error: unzip or python3 is required to extract the archive" >&2; exit 1
fi

mv "${TMP}/fnva" "${INSTALL_DIR}/fnva"
chmod +x "${INSTALL_DIR}/fnva"

# 4. wire PATH + shell integration into rc
echo ""
echo "✓ fnva installed to ${INSTALL_DIR}/fnva"
echo ""

case ":${PATH}:" in
    *":${INSTALL_DIR}:"*)
        echo "fnva bin already in PATH"
        ;;
    *)
        SHELL_NAME=$(basename "${SHELL:-sh}")
        case "$SHELL_NAME" in
            fish)
                RC="${HOME}/.config/fish/config.fish"
                mkdir -p "$(dirname "$RC")"
                if ! grep -q "fnva" "$RC" 2>/dev/null; then
                    printf '\n# >>> fnva >>>\nset -x PATH %s $PATH\nfnva env | source\n# <<< fnva <<<\n' "$INSTALL_DIR" >> "$RC"
                    echo "Added PATH + shell integration to ${RC}"
                else
                    echo "${RC} already has fnva config (skipped)"
                fi
                ;;
            *)
                ADDED=0
                for rc in "${HOME}/.zshrc" "${HOME}/.bashrc" "${HOME}/.profile"; do
                    if [ -f "$rc" ] && ! grep -q "fnva" "$rc" 2>/dev/null; then
                        printf '\n# >>> fnva >>>\nexport PATH="%s:$PATH"\neval "$(fnva env)"\n# <<< fnva <<<\n' "$INSTALL_DIR" >> "$rc"
                        echo "Added PATH + shell integration to ${rc}"
                        ADDED=1
                        break
                    fi
                done
                if [ "$ADDED" -eq 0 ]; then
                    echo "No shell rc found; add these lines to ~/.bashrc or ~/.zshrc manually:"
                    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
                    echo "  eval \"\$(fnva env)\""
                fi
                ;;
        esac
        ;;
esac

echo ""
echo "Reopen your terminal (or source your rc) and fnva is ready — verify: fnva --version"
echo "Uninstall: curl -fSsL https://raw.githubusercontent.com/${REPO}/main/scripts/uninstall.sh | sh"

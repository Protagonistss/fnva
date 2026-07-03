#!/bin/sh
# fnva uninstaller (Unix): remove binary + strip the fnva block from shell rc.
#   curl -fSsL https://raw.githubusercontent.com/Protagonistss/fnva/main/scripts/uninstall.sh | sh
#
# Keeps ~/.fnva (config + installed tools) by default; see the note at the
# end to wipe it completely.

set -eu

INSTALL_DIR="${FNVA_INSTALL_DIR:-${HOME}/.fnva/bin}"

# 1. strip the fnva block (>>> fnva >>> ... <<< fnva <<<) from rc
strip_fnva_block() {
    rc="$1"
    [ -f "$rc" ] || return 0
    if grep -q ">>> fnva >>>" "$rc" 2>/dev/null; then
        awk '/>>> fnva >>>/{skip=1; next} /<<< fnva <<</{skip=0; next} !skip' "$rc" > "${rc}.tmp" && mv "${rc}.tmp" "$rc"
        echo "Removed fnva block from ${rc}"
    fi
}

SHELL_NAME=$(basename "${SHELL:-sh}")
case "$SHELL_NAME" in
    fish) strip_fnva_block "${HOME}/.config/fish/config.fish" ;;
    *)
        for rc in "${HOME}/.zshrc" "${HOME}/.bashrc" "${HOME}/.profile"; do
            strip_fnva_block "$rc"
        done
        ;;
esac

# 2. remove binary
if [ -f "${INSTALL_DIR}/fnva" ]; then
    rm -f "${INSTALL_DIR}/fnva"
    echo "Removed ${INSTALL_DIR}/fnva"
else
    echo "${INSTALL_DIR}/fnva not found (already removed?)"
fi

# 3. note (keep ~/.fnva by default)
echo ""
echo "✓ fnva uninstalled"
echo "  Config dir ${HOME}/.fnva kept (config.toml + installed tools)."
echo "  To wipe it completely: rm -rf \"${HOME}/.fnva\""
echo ""
echo "Reopen your terminal so PATH / env changes take effect."

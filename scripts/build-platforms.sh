#!/usr/bin/env bash
# Build platform-specific fnva binaries and place them under platforms/<os>-<arch>/
# Requires the corresponding Rust targets to be installed.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

targets=(
  "aarch64-apple-darwin:darwin-arm64:fnva"
  "x86_64-apple-darwin:darwin-x64:fnva"
  "x86_64-unknown-linux-gnu:linux-x64:fnva"
  "x86_64-pc-windows-msvc:win32-x64:fnva.exe"
)

for entry in "${targets[@]}"; do
  IFS=: read -r target triple_dir bin_name <<<"${entry}"
  echo "==> Building ${target} -> platforms/${triple_dir}/${bin_name}"
  if ! command -v cargo >/dev/null 2>&1; then
    echo "!! cargo not found; install Rust toolchain first" >&2
    exit 1
  fi

  if ! rustup target list --installed | grep -q "^${target}$"; then
    echo "!! target ${target} not installed; run: rustup target add ${target}" >&2
    continue
  fi

  cargo build --release --target "${target}"

  src="${ROOT}/target/${target}/release/${bin_name}"
  dest_dir="${ROOT}/platforms/${triple_dir}"

  if [[ ! -f "${src}" ]]; then
    echo "!! Missing build output: ${src}" >&2
    continue
  fi

  mkdir -p "${dest_dir}"
  cp "${src}" "${dest_dir}/${bin_name}"
done

echo "==> Done. Binaries are in ${ROOT}/platforms/<os>-<arch>/"

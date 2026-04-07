#!/usr/bin/env bash
set -euo pipefail

REPO="V8V88V8V88/XE"
BINARY_NAME="xe"
INSTALL_DIR="${XE_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${XE_VERSION:-latest}"

log() {
  printf '%s\n' "$1"
}

fail() {
  printf 'Install error: %s\n' "$1" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1
}

detect_os() {
  case "$(uname -s)" in
    Linux) printf 'linux' ;;
    Darwin) printf 'darwin' ;;
    *)
      fail "unsupported operating system. Use GitHub Releases for manual installation."
      ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64) printf 'x86_64' ;;
    arm64|aarch64) printf 'aarch64' ;;
    *)
      fail "unsupported CPU architecture. Use GitHub Releases for manual installation."
      ;;
  esac
}

asset_name_for() {
  local os="$1"
  local arch="$2"

  case "${os}:${arch}" in
    linux:x86_64) printf 'xe-x86_64-unknown-linux-gnu.tar.gz' ;;
    darwin:x86_64) printf 'xe-x86_64-apple-darwin.tar.gz' ;;
    darwin:aarch64) printf 'xe-aarch64-apple-darwin.tar.gz' ;;
    *)
      fail "no prebuilt XE binary is published for ${os}/${arch} yet."
      ;;
  esac
}

download() {
  local url="$1"
  local output="$2"

  if need_cmd curl; then
    curl -fsSL "$url" -o "$output" || fail "failed to download ${url}. Check that the requested XE release exists and is publicly accessible."
  elif need_cmd wget; then
    wget -qO "$output" "$url" || fail "failed to download ${url}. Check that the requested XE release exists and is publicly accessible."
  else
    fail "curl or wget is required to download XE."
  fi
}

main() {
  need_cmd tar || fail "tar is required to extract XE."

  local os
  local arch
  local asset
  local base_url
  local tmp_dir
  local archive_path

  os="$(detect_os)"
  arch="$(detect_arch)"
  asset="$(asset_name_for "$os" "$arch")"

  if [ "$VERSION" = "latest" ]; then
    base_url="https://github.com/${REPO}/releases/latest/download"
  else
    base_url="https://github.com/${REPO}/releases/download/${VERSION}"
  fi

  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' EXIT
  archive_path="${tmp_dir}/${asset}"

  log "Downloading ${asset}..."
  download "${base_url}/${asset}" "$archive_path"

  mkdir -p "$INSTALL_DIR"
  tar -xzf "$archive_path" -C "$tmp_dir"
  install -m 0755 "${tmp_dir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"

  log "Installed XE to ${INSTALL_DIR}/${BINARY_NAME}"

  if ! printf '%s' "${PATH}" | tr ':' '\n' | grep -Fxq "$INSTALL_DIR"; then
    log "Add ${INSTALL_DIR} to your PATH if it is not already available in new shells."
  fi

  log "Run 'xe help' to verify the installation."
}

main "$@"

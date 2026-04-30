#!/usr/bin/env bash
set -euo pipefail

REPO="V8V88V8V88/XE"
BINARY_NAME="xe"
INSTALL_DIR="${XE_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${XE_VERSION:-latest}"
TMP_DIR=""

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

append_path_line() {
  local rc_file="$1"
  local install_dir="$2"

  if [ ! -f "$rc_file" ]; then
    touch "$rc_file"
  fi

  if grep -Fqs "$install_dir" "$rc_file"; then
    return 0
  fi

  printf '\n# Added by XE installer\nexport PATH="%s:$PATH"\n' "$install_dir" >> "$rc_file"
}

maybe_update_path() {
  local install_dir="$1"
  local updated=1

  # Update all common shell config files that exist
  for rc_file in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.profile"; do
    if [ -f "$rc_file" ]; then
      append_path_line "$rc_file" "$install_dir"
      log "Added PATH entry to ${rc_file}"
      updated=0
    fi
  done

  # If none of the files existed, fall back to creating one based on the shell
  if [ "$updated" -eq 1 ]; then
    local fallback=""
    case "$(basename "${SHELL:-}")" in
      zsh) fallback="$HOME/.zshrc" ;;
      bash)
        if [ "$(detect_os)" = "darwin" ]; then
          fallback="$HOME/.bash_profile"
        else
          fallback="$HOME/.bashrc"
        fi
        ;;
      *) fallback="$HOME/.profile" ;;
    esac

    append_path_line "$fallback" "$install_dir"
    log "Added PATH entry to ${fallback}"
  fi

  log "Open a new terminal to start using XE."
  return 0
}

cleanup() {
  if [ -n "${TMP_DIR}" ] && [ -d "${TMP_DIR}" ]; then
    rm -rf "${TMP_DIR}"
  fi
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
  local archive_path

  os="$(detect_os)"
  arch="$(detect_arch)"
  asset="$(asset_name_for "$os" "$arch")"

  if [ "$VERSION" = "latest" ]; then
    base_url="https://github.com/${REPO}/releases/latest/download"
  else
    base_url="https://github.com/${REPO}/releases/download/${VERSION}"
  fi

  TMP_DIR="$(mktemp -d)"
  trap cleanup EXIT
  archive_path="${TMP_DIR}/${asset}"

  log "Downloading ${asset}..."
  download "${base_url}/${asset}" "$archive_path"

  mkdir -p "$INSTALL_DIR"
  tar -xzf "$archive_path" -C "$TMP_DIR"
  install -m 0755 "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"

  log "Installed XE to ${INSTALL_DIR}/${BINARY_NAME}"

  if ! printf '%s' "${PATH}" | tr ':' '\n' | grep -Fxq "$INSTALL_DIR"; then
    if ! maybe_update_path "$INSTALL_DIR"; then
      log "Add ${INSTALL_DIR} to your PATH if it is not already available in new shells."
    fi
  fi

  log "Run 'xe help' to verify the installation."
}

main "$@"

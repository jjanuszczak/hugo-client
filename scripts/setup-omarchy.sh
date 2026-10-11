#!/usr/bin/env bash
set -euo pipefail

if [[ ! -r /etc/os-release ]]; then
  echo "Cannot identify this Linux distribution (/etc/os-release is missing)." >&2
  exit 1
fi

# shellcheck disable=SC1091
source /etc/os-release
if [[ "${ID:-}" != "omarchy" && " ${ID_LIKE:-} " != *" arch "* ]]; then
  echo "This setup script is for Omarchy and Arch-based Linux; detected '${PRETTY_NAME:-unknown}'." >&2
  exit 1
fi

if ! command -v pacman >/dev/null 2>&1; then
  echo "pacman is required to install the system packages." >&2
  exit 1
fi

if ! command -v sudo >/dev/null 2>&1; then
  echo "sudo is required to install system packages." >&2
  exit 1
fi

sudo pacman -S --needed \
  webkit2gtk-4.1 \
  base-devel \
  curl \
  wget \
  file \
  openssl \
  appmenu-gtk-module \
  libappindicator-gtk3 \
  librsvg \
  xdotool \
  rustup

# Arch's rustup package provides the manager; select the stable compiler and
# Cargo toolchain needed by Tauri.
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"
rustup default stable

echo
echo "Omarchy/Tauri prerequisites are installed. Verify with: rustc --version && cargo --version"
echo "Build Linux bundles with: npx tauri build --bundles deb,appimage"

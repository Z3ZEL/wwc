#!/bin/sh
# Installs what scripts/build-frontend.sh needs, on a fresh Linux x86_64 machine (CI, static host build step).
# Idempotent: tools already present are left alone.
# Usage: scripts/install-frontend-tools.sh && WWC_API_URL=https://api.example.com scripts/build-frontend.sh
#   CARGO_HOME     where rustup, cargo and trunk go (default ~/.cargo)
#   TRUNK_VERSION  Trunk release (default below; keep in lockstep with frontend/Dockerfile.dev)
# Needs curl, tar and a C linker (cc), which build scripts and proc macros compile against.
set -eu
TRUNK_VERSION="${TRUNK_VERSION:-0.21.14}"
CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
export CARGO_HOME
PATH="$CARGO_HOME/bin:$PATH"
cd "$(dirname "$0")/.."

for tool in curl tar cc; do
  command -v "$tool" >/dev/null 2>&1 || { echo "error: '$tool' is required but not installed" >&2; exit 1; }
done

if ! command -v rustup >/dev/null 2>&1; then
  echo "Installing rustup..."
  curl -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain none
fi
# Installs the toolchain, wasm target and components pinned in rust-toolchain.toml.
rustup toolchain install

if [ "$(trunk --version 2>/dev/null)" != "trunk $TRUNK_VERSION" ]; then
  echo "Installing trunk $TRUNK_VERSION..."
  mkdir -p "$CARGO_HOME/bin"
  curl -sSfL "https://github.com/trunk-rs/trunk/releases/download/v$TRUNK_VERSION/trunk-x86_64-unknown-linux-gnu.tar.gz" \
    | tar -xz -C "$CARGO_HOME/bin" trunk
fi
trunk --version

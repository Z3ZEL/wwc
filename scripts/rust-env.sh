# Sourced by the frontend build scripts: points rustup and cargo at writable homes.
# Some build hosts (e.g. Render) preinstall rustup under a read-only /usr/local; the preinstalled
# rustup/cargo binaries still work, but toolchains, the registry and trunk then go under $HOME.
writable_dir() { mkdir -p "$1" 2>/dev/null && [ -w "$1" ]; }
RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
writable_dir "$RUSTUP_HOME" || RUSTUP_HOME="$HOME/.rustup"
CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
writable_dir "$CARGO_HOME" || CARGO_HOME="$HOME/.cargo"
export RUSTUP_HOME CARGO_HOME
PATH="$CARGO_HOME/bin:$PATH"

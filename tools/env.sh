W="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export RUSTUP_HOME="$W/tools/rust/rustup"
export CARGO_HOME="$W/tools/rust/cargo"
export PATH="$CARGO_HOME/bin${MINGW:+:$MINGW}:$PATH"
export CC=gcc AR=ar

# source this from Git Bash: puts the local Rust toolchain and MinGW-w64 gcc on PATH
W="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export RUSTUP_HOME="$W/tools/rust/rustup"
export CARGO_HOME="$W/tools/rust/cargo"
# MinGW-w64 gcc 16 + binutils (portable MSYS2 set up for the FE7 project)
MINGW="${MINGW:-$(cd "$W/../../FE7/CLAUDE/tools/msys64/mingw64/bin" 2>/dev/null && pwd)}"
export PATH="$CARGO_HOME/bin:$MINGW:$PATH"
export CC=gcc AR=ar

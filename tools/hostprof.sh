#!/usr/bin/env bash
# host-side sampling profile of a stretch of the game, from a save state:
#
#   tools/hostprof.sh <state> <frames> <seconds> <name> [ENV=VALUE ...]
#
# runs the boot harness on the recompiled DLL from the state, waits for it
# to load, samples its threads for <seconds> and writes the report to
# _runs/<name>/profile.txt (see tools/sample_profile.py).
set -u
W="$(cd "$(dirname "$0")/.." && pwd)"
PY="$(command -v python)"
source "$W/tools/env.sh"
state="$1" frames="$2" seconds="$3" name="$4"; shift 4
out="$W/_runs/$name"
rm -rf "$out"; mkdir -p "$out"
export BINUTILS="$(cygpath -w "$MINGW")"
export TMP="$(cygpath -w "$out")" TEMP="$(cygpath -w "$out")"
export ZAKURO_RASTERIZER="${ZAKURO_RASTERIZER:-hardware}"
export ZAKURO_LOAD="$(cygpath -m "$state")"
export ZAKURO_DATA="$(cygpath -m "$W/_runs/states/data-avatar")"
export ZAKURO_RECOMPILED="${DLL:-$(cygpath -m "$W/FireEmblemAwakeningRecomp/code/000400000009F100.dll")}"
for setting in "$@"; do export "$setting"; done
exe="$(cygpath -w "$W/zakuro/target/release/examples/boot.exe")"
rom="$(cygpath -w "$W/../Fire Emblem - Awakening (Europe) (En,Fr,De,Es,It).cci")"
"$PY" - "$exe" "$rom" "$frames" "$seconds" "$(cygpath -w "$out")" "$(cygpath -w "$W/tools/sample_profile.py")" <<'EOF'
import os, runpy, subprocess, sys, time
exe, rom, frames, seconds, out, profiler = sys.argv[1:7]
process = subprocess.Popen([exe, rom, frames], stdout=open(os.path.join(out, "log.txt"), "w"), stderr=subprocess.STDOUT)
time.sleep(4)
sys.argv = ["sample_profile.py", str(process.pid), seconds, os.path.join(out, "profile.txt")]
runpy.run_path(profiler, run_name="__main__")
process.wait()
EOF
grep -E "^outcome|interpreted" "$out/log.txt"
head -60 "$out/profile.txt"

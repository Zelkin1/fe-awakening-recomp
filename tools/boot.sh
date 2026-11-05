#!/usr/bin/env bash
# headless test run of Awakening in Zakuro's boot harness, isolated from any
# real save: saves go to _runs/<name>/data (or DATA=dir), screen dumps to
# _runs/<name>/ as PNG.
#
#   tools/boot.sh <frames> [name]
#
# env: INPUT=300:A,420:DOWN+A   buttons to hold from those frames (ZAKURO_INPUT)
#      DUMP_AT=600,900          extra frames to save the screens at
#      INTERP=1                 interpret everything, no recompiled code
#      DLL=path                 another recompiled library
#      any other ZAKURO_* variable passes straight through
set -u
W="$(cd "$(dirname "$0")/.." && pwd)"
# the python with PIL, before MinGW's own comes first on PATH
PY="${PY:-$(command -v python)}"
source "$W/tools/env.sh"
frames="${1:-600}"
name="${2:-run}"
out="$W/_runs/$name"
rm -rf "$out"; mkdir -p "$out"

# a fresh save each run unless DATA names a folder to keep one in
export ZAKURO_DATA="${DATA:-$out/data}"
mkdir -p "$ZAKURO_DATA"
export TMP="$out" TEMP="$out"
[ -n "${INPUT:-}" ] && export ZAKURO_INPUT="$INPUT"
[ -n "${DUMP_AT:-}" ] && export ZAKURO_DUMP_AT="$DUMP_AT"
if [ -z "${INTERP:-}" ]; then
    export ZAKURO_RECOMPILED="${DLL:-$W/FireEmblemAwakeningRecomp/code/000400000009F100.dll}"
fi

rom="$W/../Fire Emblem - Awakening (Europe) (En,Fr,De,Es,It).cci"
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"
"$W/zakuro/target/release/examples/boot.exe" "$rom" "$frames" 2>&1 | tee "$out/log.txt"
status=${PIPESTATUS[0]}
echo "exit status $status" | tee -a "$out/log.txt"

"$PY" "$W/tools/ppm2png.py" "$out"
exit $status

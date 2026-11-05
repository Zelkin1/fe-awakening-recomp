#!/usr/bin/env bash
# headless smoke test of the built port, fe-awakening.exe, isolated from the
# player's saves and settings: data and APPDATA both go to _runs/<name>/.
#
#   tools/port-headless.sh [frames] [name]
set -u
W="$(cd "$(dirname "$0")/.." && pwd)"
WM="$(cygpath -m "$W")"
frames="${1:-900}"
name="${2:-port}"
out="$W/_runs/$name"
rm -rf "$out"; mkdir -p "$out/data" "$out/appdata"

# Zakuro writes settings.toml under %APPDATA% whatever --data says
export APPDATA="$(cygpath -w "$out/appdata")"
rom="$WM/../Fire Emblem - Awakening (Europe) (En,Fr,De,Es,It).cci"
"$W/FireEmblemAwakeningRecomp/target/release/fe-awakening.exe" "$rom" --headless "$frames" --data "$WM/_runs/$name/data" 2>&1 | tee "$out/log.txt"
status=${PIPESTATUS[0]}
echo "exit status $status" | tee -a "$out/log.txt"
exit $status

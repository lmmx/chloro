#!/bin/bash
# EXP-029: several traced binaries on the nine crate workloads, interleaved in blocks, so that
# each build's traces come from the same load window.
# usage: [CRATES="..."] collect_ab.sh OUT BLOCKS RUNS_PER_BLOCK NAME=BINARY...
#   per crate, BLOCKS times: RUNS_PER_BLOCK runs of each binary in turn (diag.py, --check
#   --no-cache, 4 threads, unpinned); traces and runs in OUT/NAME/<crate>.b<k>.*, the load in OUT/log.
# Read with: split_sim.py --compare OUT NAME:POLICY NAME:POLICY
set -eu
OUT=$1; NB=$2; R=$3; shift 3
HERE=$(cd "$(dirname "$0")" && pwd)
CRATES=${CRATES:-"fx-hir fx-ide-db fx-hir-ty fx-syntax fx-parser reg-tokio reg-regex-syntax reg-clap-builder reg-winnow"}
mkdir -p "$OUT"; : >> "$OUT/log"
for c in $CRATES; do
  for k in $(seq 0 $((NB-1))); do
    for nb in "$@"; do
      name=${nb%%=*}; bin=${nb#*=}
      mkdir -p "$OUT/$name"
      echo "$name $c b$k $(date -u +%T) load $(cut -d' ' -f1-3 /proc/loadavg)" >> "$OUT/log"
      E23_MODE=check-nocache python3 -I "$HERE/diag.py" "$bin" /tmp/chloro-bench/formatted/$c/tree $R 4 "$OUT/$name/$c.b$k" >> "$OUT/log"
    done
  done
done
echo "done $(date -u +%T) load $(cut -d' ' -f1-3 /proc/loadavg)" >> "$OUT/log"

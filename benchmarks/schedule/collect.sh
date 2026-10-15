#!/bin/bash
# EXP-023: run the traced CLI on the nine crate workloads and write one results directory per
# configuration. usage: collect.sh BINARY OUT [RUNS4] [RUNS1]
#   OUT/base4  --check --no-cache, 4 threads, unpinned       (the schedule diagnostic)
#   OUT/base1  --check --no-cache, 1 thread, taskset -c 3    (phases on one core)
#   OUT/cold4, OUT/cold1   --check with a fresh cache directory per run (hashing, cache open/save)
#   OUT/write4, OUT/write1 --write --no-cache on a fresh copy of the raw tree per run
# Each directory holds <crate>.trace<k>.tsv and <crate>.runs.tsv; OUT/<cfg>.log holds the
# medians. BINARY must read CHLORO_TRACE (any build since EXP-023); for the reconstruction
# (sim.py) it must also carry trace_patch.py's thread CPU clocks.
set -eu
B=$1; OUT=$2; R4=${3:-20}; R1=${4:-10}
HERE=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$OUT/tmp"; export TMPDIR="$OUT/tmp"
CRATES="fx-hir fx-hir-ty fx-ide-db fx-syntax fx-parser reg-tokio reg-regex-syntax reg-clap-builder reg-winnow"
one() { # cfg mode runs threads [cpus]
  local cfg=$1 mode=$2 runs=$3 th=$4 cpus=${5:-}
  mkdir -p "$OUT/$cfg"; : > "$OUT/$cfg.log"
  for c in $CRATES; do
    echo "$c $(date -u +%T) load $(cut -d' ' -f1-3 /proc/loadavg)" >> "$OUT/$cfg.log"
    E23_MODE=$mode python3 -I "$HERE/diag.py" "$B" /tmp/chloro-bench/formatted/$c/tree $runs $th "$OUT/$cfg/$c" $cpus >> "$OUT/$cfg.log"
  done
}
one base4 check-nocache $R4 4
one base1 check-nocache $R1 1 3
one cold4 check-coldcache 5 4
one cold1 check-coldcache 5 1 3
one write4 write-raw 5 4
one write1 write-raw 5 1 3

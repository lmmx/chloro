#!/usr/bin/env bash
# Cliff hunt driver (EXP-019): generate the scaling fixtures, check them against rustfmt,
# count the instructions chloro spends on each, and fit the exponents.
#
#   benchmarks/cliffs/run.sh [--bench BIN] [--stdin BIN] [--core N] [--scales 1,2,4,8]
#                            [--work DIR] [--shapes 'a b ..'] [--no-compare]
#
# Writes WORK/fixtures/<shape>/s<k>/<shape>.rs (gen.py), WORK/compare.tsv (rustfmt exit
# status, chloro output identical to rustfmt's or not), WORK/ir.tsv (shape, scale, input
# bytes, Ir, chloro's output bytes) and WORK/cg/<shape>.s<k>.out (callgrind profiles), then prints `gen.py fit`.
# The callgrind invocation is profile.sh's: `env -i`, `setarch -R`, pinned with taskset.
set -euo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
BENCH=$REPO/target/release/examples/bench
STDIN=$REPO/target/release/examples/fmt_stdin
CORE= SCALES=1,2,4,8 WORK=${TMPDIR:-/tmp}/chloro-bench/cliffs SHAPES= COMPARE=1

die() { echo "run.sh: error: $*" >&2; exit 2; }
while [ $# -gt 0 ]; do
  case $1 in
    --bench) BENCH=$2; shift ;;
    --stdin) STDIN=$2; shift ;;
    --core) CORE=$2; shift ;;
    --scales) SCALES=$2; shift ;;
    --work) WORK=$2; shift ;;
    --shapes) SHAPES=$2; shift ;;
    --no-compare) COMPARE=0 ;;
    -h|--help) sed -n '2,12p' "$0"; exit 0 ;;
    *) die "unknown argument $1" ;;
  esac
  shift
done

for t in valgrind setarch python3; do command -v "$t" >/dev/null || die "required tool '$t' not found"; done
[ -z "$CORE" ] || command -v taskset >/dev/null || die "taskset not found"
VALGRIND=$(command -v valgrind) SETARCH=$(command -v setarch)
BENCH=$(cd "$(dirname "$BENCH")" && pwd)/$(basename "$BENCH")
[ -x "$BENCH" ] || die "$BENCH not found; build with: cargo build --release -p chloro-core --examples"

mkdir -p "$WORK"
WORK=$(cd "$WORK" && pwd)
FIX=$WORK/fixtures
rm -rf "$FIX" "$WORK/cg"; mkdir -p "$WORK/cg"
python3 "$HERE/gen.py" generate "$FIX" --scales "$SCALES"
[ -n "$SHAPES" ] || SHAPES=$(python3 "$HERE/gen.py" list | cut -f1)
echo "bench: $BENCH sha256=$(sha256sum "$BENCH" | cut -c1-16)"

if [ "$COMPARE" = 1 ]; then
  command -v rustfmt >/dev/null || die "rustfmt not found"
  [ -x "$STDIN" ] || die "$STDIN not found"
  printf 'shape\tscale\trustfmt_exit\tidentical\n' > "$WORK/compare.tsv"
  for shape in $SHAPES; do
    for s in ${SCALES//,/ }; do
      f=$FIX/$shape/s$s/$shape.rs
      rc=0; rustfmt --edition 2024 < "$f" > "$WORK/expected.rs" 2>/dev/null || rc=$?
      "$STDIN" < "$f" > "$WORK/actual.rs"
      same=no; cmp -s "$WORK/expected.rs" "$WORK/actual.rs" && same=yes
      printf '%s\t%s\t%s\t%s\n' "$shape" "$s" "$rc" "$same" >> "$WORK/compare.tsv"
    done
  done
  rm -f "$WORK/expected.rs" "$WORK/actual.rs"
  awk -F'\t' 'NR > 1 && ($3 != 0 || $4 != "yes")' "$WORK/compare.tsv" | sed 's/^/compare: /'
fi

pin=()
[ -n "$CORE" ] && pin=(taskset -c "$CORE")
count() {  # count NAME SCALE DIR
  local cg=$WORK/cg/$1.s$2.out log=$WORK/cg/$1.s$2.log
  "${pin[@]}" env -i "$SETARCH" "$(uname -m)" -R "$VALGRIND" --tool=callgrind \
    --callgrind-out-file="$cg" "$BENCH" -n 1 --root "$3" > /dev/null 2> "$log" \
    || { cat "$log" >&2; die "valgrind failed on $1 s$2"; }
  sed -n 's/.*Collected : *//p' "$log" | tr -d ',' | head -1
}
outbytes() { if [ -x "$STDIN" ]; then "$STDIN" < "$1" | wc -c; else echo 0; fi; }
printf 'shape\tscale\tbytes\tir\tout_bytes\n' > "$WORK/ir.tsv"
printf 'empty\t1\t0\t%s\t0\n' "$(count empty 1 "$FIX/empty/s1")" >> "$WORK/ir.tsv"
for shape in $SHAPES; do
  for s in ${SCALES//,/ }; do
    d=$FIX/$shape/s$s
    printf '%s\t%s\t%s\t%s\t%s\n' "$shape" "$s" "$(wc -c < "$d/$shape.rs")" \
      "$(count "$shape" "$s" "$d")" "$(outbytes "$d/$shape.rs")" >> "$WORK/ir.tsv"
  done
done
python3 "$HERE/gen.py" fit "$WORK/ir.tsv"

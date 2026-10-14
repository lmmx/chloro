#!/usr/bin/env bash
# Instruction count (callgrind total Ir) of examples/bench formatting the fixed 49-file
# sample once. Deterministic to within a few thousand instructions since D1 (fc90e53), so
# it resolves changes far below wall-clock noise.
#
#   benchmarks/profile.sh [--bench BIN] [--runs N] [--core N] [--out DIR] [--annotate N]
#
# The sample: the 1238 `.rs` files under chloro-core/tests/conformance/fixtures, paths
# sorted bytewise (`LC_ALL=C sort`), every 25th file (lines 25, 50, ..., 1225): 49 files.
# The journals record "every 25th file of the sorted fixture list" but not the command;
# this construction is the one of five tried whose count (588.0M at 884f9c9) lies near the
# last recorded figure (586.8M at f6c5be9). See benchmarks/README.md.
set -euo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=$(cd "$HERE/.." && pwd)
FIXTURES=$REPO/chloro-core/tests/conformance/fixtures
BENCH=$REPO/target/release/examples/bench
RUNS=1 CORE= OUT= ANNOTATE=0
SAMPLE=/tmp/chloro-bench/sample49

die() { echo "profile.sh: error: $*" >&2; exit 2; }
while [ $# -gt 0 ]; do
  case $1 in
    --bench) BENCH=$2; shift ;;
    --runs) RUNS=$2; shift ;;
    --core) CORE=$2; shift ;;
    --out) OUT=$2; shift ;;
    --sample) SAMPLE=$2; shift ;;
    --annotate) ANNOTATE=$2; shift ;;
    -h|--help) sed -n '2,15p' "$0"; exit 0 ;;
    *) die "unknown argument $1" ;;
  esac
  shift
done

for t in valgrind setarch; do command -v "$t" >/dev/null || die "required tool '$t' not found"; done
[ "$ANNOTATE" = 0 ] || command -v callgrind_annotate >/dev/null || die "callgrind_annotate not found"
[ -z "$CORE" ] || command -v taskset >/dev/null || die "taskset not found"
VALGRIND=$(command -v valgrind) SETARCH=$(command -v setarch)
BENCH=$(cd "$(dirname "$BENCH")" && pwd)/$(basename "$BENCH")
[ -x "$BENCH" ] || die "$BENCH not found; build with: cargo build --release -p chloro-core --examples"
[ -d "$FIXTURES" ] || die "$FIXTURES not found"
[ -n "$OUT" ] || OUT=$(mktemp -d "${TMPDIR:-/tmp}/chloro-callgrind.XXXXXX")
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)

# Build the sample.
rm -rf "$SAMPLE"; mkdir -p "$SAMPLE"
(cd "$FIXTURES" && find . -name '*.rs' | sed 's|^\./||' | LC_ALL=C sort | awk 'NR % 25 == 0') > "$OUT/sample.txt"
total=$(cd "$FIXTURES" && find . -name '*.rs' | wc -l)
[ "$total" = 1238 ] || echo "warning: fixture list has $total files, not 1238; the sample differs from the journals'" >&2
(cd "$FIXTURES" && xargs -a "$OUT/sample.txt" cp --parents -t "$SAMPLE")
n=$(wc -l < "$OUT/sample.txt")
bytes=$(cd "$SAMPLE" && xargs -a "$OUT/sample.txt" cat | wc -c)
echo "sample: $n files, $bytes bytes ($SAMPLE; list in $OUT/sample.txt)"
echo "bench: $BENCH sha256=$(sha256sum "$BENCH" | cut -c1-16)"

pin=()
[ -n "$CORE" ] && pin=(taskset -c "$CORE")
irs=()
for ((i = 1; i <= RUNS; i++)); do
  cg=$OUT/callgrind.$i.out
  log=$OUT/callgrind.$i.log
  # Fixed environment and sample path: the count moves by up to ~0.1% with the size of the
  # environment and the path strings (stack and heap layout), not with the code.
  "${pin[@]}" env -i "$SETARCH" "$(uname -m)" -R "$VALGRIND" --tool=callgrind --callgrind-out-file="$cg" \
    "$BENCH" -n 1 --root "$SAMPLE" > "$OUT/bench.$i.txt" 2> "$log" || { cat "$log" >&2; die "valgrind failed"; }
  ir=$(sed -n 's/.*Collected : *//p' "$log" | tr -d ',' | head -1)
  [ -n "$ir" ] || { cat "$log" >&2; die "no 'Collected' line in $log"; }
  irs+=("$ir")
  printf 'run %d: Ir %s\n' "$i" "$ir"
done
min=$(printf '%s\n' "${irs[@]}" | sort -n | head -1)
max=$(printf '%s\n' "${irs[@]}" | sort -n | tail -1)
printf 'Ir min %s  max %s  (%d runs; profiles in %s)\n' "$min" "$max" "$RUNS" "$OUT"
if [ "$ANNOTATE" != 0 ]; then
  callgrind_annotate --threshold=100 "$OUT/callgrind.1.out" 2>/dev/null | sed -n '1,/^--.*$/!p' | head -n "$((ANNOTATE + 25))"
fi

#!/usr/bin/env bash
# Instruction count (callgrind total Ir) of examples/bench formatting the fixed 49-file
# sample once. Deterministic to within about 400 instructions since D1 (fc90e53) (about one
# run in five reads 1.8K more, from bench's own timing report; see benchmarks/README.md) and
# independent of where the sample and the binary live (below), so it resolves changes far
# below wall-clock noise.
#
#   benchmarks/profile.sh [--bench BIN] [--runs N] [--core N] [--out DIR] [--annotate N] [--sim]
#                         [--per-file] [--sample-list FILE [--sample-root DIR]] [--phase parse]
#
# --sim adds callgrind's cache and branch simulation (--cache-sim=yes --branch-sim=yes) and
# prints, per run, Ir, Bc, Bcm, Bi, Bim, I1mr, D1mr, D1mw and the ten functions with the
# most I1mr and the most Bcm (exclusive). Ir is the same with and without it (within the
# run-to-run spread).
#
# --per-file also formats each file of the sample in a bench process of its own (a fresh heap
# per file) and an empty sample (the fixed cost of a process), and prints each file's Ir,
# their sum and both figures net of the fixed cost beside the whole sample's; the
# difference is the cross-file term (the heap layout one file leaves the next; EXP-019,
# EXP-020). With --runs N each file's figure is its minimum over the N runs.
#
# --sample-list FILE builds the sample from a list of paths, one per line ('#' lines are
# comments), relative to the root the list names in a '# root: cargo-registry' line
# (${CARGO_HOME:-~/.cargo}/registry/src) or '# root: fixtures', or to --sample-root DIR.
# benchmarks/samples/registry49.txt is the registry validation sample (protocol.md section 3).
#
# --phase parse runs `bench --phase parse`: every file parsed as formatting parses it
# (chloro_core::parse_only) and not formatted; with --per-file, per file too.
#
# The default sample: the 1238 `.rs` files under chloro-core/tests/conformance/fixtures, paths
# sorted bytewise (`LC_ALL=C sort`), every 25th file (lines 25, 50, ..., 1225): 49 files.
# The journals record "every 25th file of the sorted fixture list" but not the command;
# this construction is the one of five tried whose count (588.0M at 884f9c9) lies near the
# last recorded figure (586.8M at f6c5be9). See benchmarks/README.md.
#
# Path independence (EXP-027): the binary and the sample are copied under a directory whose
# path has a fixed length (mktemp -d /tmp/chloro-prof.XXXXXXX), so the count does not depend
# on where BIN, the sample or OUT live. The two paths bench sees have the lengths of the
# ones this script used before (47 and 26 bytes), so the count is the one recorded then.
set -euo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=$(cd "$HERE/.." && pwd)
FIXTURES=$REPO/chloro-core/tests/conformance/fixtures
BENCH=$REPO/target/release/examples/bench
RUNS=1 CORE= OUT= ANNOTATE=0 SIM=0 PERFILE=0 LIST= LIST_ROOT= PHASE=format

die() { echo "profile.sh: error: $*" >&2; exit 2; }
while [ $# -gt 0 ]; do
  case $1 in
    --bench) BENCH=$2; shift ;;
    --runs) RUNS=$2; shift ;;
    --core) CORE=$2; shift ;;
    --out) OUT=$2; shift ;;
    --sample) echo "profile.sh: note: --sample is ignored: the sample is staged under a path of fixed length (EXP-027)" >&2; shift ;;
    --annotate) ANNOTATE=$2; shift ;;
    --sim) SIM=1; shift; continue ;;
    --per-file) PERFILE=1; shift; continue ;;
    --sample-list) LIST=$2; shift ;;
    --sample-root) LIST_ROOT=$2; shift ;;
    --phase) PHASE=$2; shift ;;
    -h|--help) sed -n '2,39p' "$0"; exit 0 ;;
    *) die "unknown argument $1" ;;
  esac
  shift
done

case $PHASE in format | parse) ;; *) die "--phase takes format or parse, not '$PHASE'" ;; esac
for t in valgrind setarch; do command -v "$t" >/dev/null || die "required tool '$t' not found"; done
[ "$ANNOTATE" = 0 ] && [ "$SIM" = 0 ] || command -v callgrind_annotate >/dev/null || die "callgrind_annotate not found"
[ -z "$CORE" ] || command -v taskset >/dev/null || die "taskset not found"
VALGRIND=$(command -v valgrind) SETARCH=$(command -v setarch)
BENCH=$(cd "$(dirname "$BENCH")" && pwd)/$(basename "$BENCH")
[ -x "$BENCH" ] || die "$BENCH not found; build with: cargo build --release -p chloro-core --examples"
[ -d "$FIXTURES" ] || die "$FIXTURES not found"
[ -n "$OUT" ] || OUT=$(mktemp -d "${TMPDIR:-/tmp}/chloro-callgrind.XXXXXX")
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)

# What bench sees (EXP-027). The count depends on the lengths of the strings the process
# holds, not on their text: argv[0] and the path strings through malloc's size classes (the
# heap layout the parser's vector growth meets, EXP-020) and the stack, and the binary's
# own path through glibc's reading of /proc/self/maps at start-up (pthread_getattr_np,
# about 40 instructions per byte). So everything bench sees lives under STAGE, a directory
# of fixed length (24 bytes) in /tmp: the binary as STAGE/bench_..., 47 bytes, as
# /home/user/chloro/target/release/examples/bench was, and the sample as STAGE/s, 26
# bytes, as /tmp/chloro-bench/sample49 was. The count at the old default is unchanged.
# --per-file stages each file in turn, and the empty sample, at STAGE/f (26 bytes too).
STAGE=$(mktemp -d /tmp/chloro-prof.XXXXXXX)
trap 'rm -rf "$STAGE"' EXIT
fixed() { local s=$1; while [ ${#s} -lt "$2" ]; do s+=_; done; printf '%s' "$s"; }
EXE=$STAGE/$(fixed bench $((47 - ${#STAGE} - 1)))
ROOT=$STAGE/s ONE=$STAGE/f
[ ${#EXE} = 47 ] && [ ${#ROOT} = 26 ] || die "staging paths $EXE, $ROOT are not 47 and 26 bytes long"
cp "$BENCH" "$EXE"

# Build the sample: OUT/sample.txt lists its files relative to SRC.
if [ -n "$LIST" ]; then
  [ -f "$LIST" ] || die "$LIST not found"
  if [ -z "$LIST_ROOT" ]; then
    case $(sed -n 's/^# root: *//p' "$LIST" | head -1) in
      cargo-registry) LIST_ROOT=${CARGO_HOME:-$HOME/.cargo}/registry/src ;;
      fixtures) LIST_ROOT=$FIXTURES ;;
      '') die "$LIST names no root ('# root: cargo-registry' or '# root: fixtures'); give --sample-root" ;;
      *) die "$LIST: unknown root '$(sed -n 's/^# root: *//p' "$LIST" | head -1)'" ;;
    esac
  fi
  [ -d "$LIST_ROOT" ] || die "$LIST_ROOT not found"
  SRC=$(cd "$LIST_ROOT" && pwd)
  grep -v '^#' "$LIST" | sed '/^[[:space:]]*$/d' > "$OUT/sample.txt"
  while read -r f; do
    case $f in /* | *..*) die "$LIST: '$f' is not a relative path below the root" ;; esac
    [ -f "$SRC/$f" ] || die "$SRC/$f not found (listed in $LIST)"
  done < "$OUT/sample.txt"
else
  SRC=$FIXTURES
  (cd "$FIXTURES" && find . -name '*.rs' | sed 's|^\./||' | LC_ALL=C sort | awk 'NR % 25 == 0') > "$OUT/sample.txt"
  total=$(cd "$FIXTURES" && find . -name '*.rs' | wc -l)
  [ "$total" = 1238 ] || echo "warning: fixture list has $total files, not 1238; the sample differs from the journals'" >&2
fi
mkdir -p "$ROOT"
(cd "$SRC" && xargs -d '\n' -a "$OUT/sample.txt" cp --parents -t "$ROOT")
n=$(wc -l < "$OUT/sample.txt")
bytes=$(cd "$ROOT" && xargs -d '\n' -a "$OUT/sample.txt" cat | wc -c)
echo "sample: $n files, $bytes bytes from $SRC (list in $OUT/sample.txt)"
echo "bench: $BENCH sha256=$(sha256sum "$BENCH" | cut -c1-16)"
# The format phase passes no option, so its count is the one recorded before --phase existed.
phase=()
if [ "$PHASE" = parse ]; then
  phase=(--phase parse)
  echo "phase: parse only (bench --phase parse)"
fi

# The simulator's totals of one profile, then the top ten functions by I1mr and by Bcm.
sim_report() {
  awk '/^events:/ { for (i = 2; i <= NF; i++) ev[i] = $i }
       /^(totals|summary):/ { for (i = 2; i <= NF; i++) v[ev[i]] = $i }
       END {
         if (!("Bcm" in v) || !("I1mr" in v)) { print "  no simulator events in the profile"; exit 1 }
         printf "  Bc %d  Bcm %d (%.2f%%)  Bi %d  Bim %d (%.2f%%)  Bcm+Bim %d\n", v["Bc"], v["Bcm"],
           100 * v["Bcm"] / (v["Bc"] ? v["Bc"] : 1), v["Bi"], v["Bim"], 100 * v["Bim"] / (v["Bi"] ? v["Bi"] : 1),
           v["Bcm"] + v["Bim"]
         printf "  I1mr %d (%.3f%% of Ir)  D1mr %d  D1mw %d  ILmr %d  DLmr %d  DLmw %d\n", v["I1mr"],
           100 * v["I1mr"] / v["Ir"], v["D1mr"], v["D1mw"], v["ILmr"], v["DLmr"], v["DLmw"]
       }' "$1" || return 0
  local ev
  for ev in I1mr Bcm; do
    echo "  top 10 by $ev (exclusive):"
    callgrind_annotate --show="$ev" --sort="$ev" --threshold=100 "$1" 2>/dev/null |
      awk '/^-+$/ { n++; next } n >= 3 && NF > 1 && $1 ~ /^[0-9,]+$/' |
      sed -n '2,11p' | sed -E 's/ \[[^]]*\]$//; s/^( *[0-9,]+ +\( *[0-9.]+%\) +)(\?\?\?|[^ ]*\.[cS]):/\1/; s/^/    /'
  done
}

pin=()
[ -n "$CORE" ] && pin=(taskset -c "$CORE")
# One callgrind run of bench on a staged root; prints the total Ir.
#   count_ir ROOT PROFILE BENCH_OUTPUT LOG FILES [VALGRIND_OPTION...]
# Fixed environment (env -i), address space (setarch -R) and path lengths (EXE, ROOT): the
# count moves by up to 1% with the size of the environment and the paths' lengths (stack and
# heap layout), not with the code.
count_ir() {
  local root=$1 cg=$2 txt=$3 log=$4 files=$5 seen ir
  shift 5
  (cd "$STAGE" && "${pin[@]}" env -i "$SETARCH" "$(uname -m)" -R "$VALGRIND" --tool=callgrind "$@" \
    --callgrind-out-file="$cg" "$EXE" -n 1 --root "$root" ${phase[@]+"${phase[@]}"}) < /dev/null > "$txt" 2> "$log" ||
    { cat "$log" >&2; die "valgrind failed (log in $log)"; }
  seen=$(sed -n 's/^\([0-9]*\) files,.*/\1/p' "$txt")
  [ "$seen" = "$files" ] || die "bench formatted ${seen:-no} files, not $files (output in $txt)"
  # Ir is the first of the collected events, with or without --sim.
  ir=$(sed -n 's/.*Collected : *//p' "$log" | tr -d ',' | head -1 | awk '{print $1}')
  [ -n "$ir" ] || { cat "$log" >&2; die "no 'Collected' line in $log"; }
  printf '%s' "$ir"
}

# --per-file, run I: the empty sample, then each file of the sample alone, each in its own
# process, staged at ONE; writes OUT/perfile.I.tsv (index, Ir, bytes, path; index 0 is the
# empty sample) and keeps the profiles in OUT/perfile.I/.
per_file_run() {
  local d=$OUT/perfile.$1 k=0 f ir
  mkdir -p "$d"
  rm -rf "$ONE"; mkdir -p "$ONE"
  ir=$(count_ir "$ONE" "$d/0.out" "$d/0.txt" "$d/0.log" 0)
  printf '0\t%s\t0\t(empty sample)\n' "$ir" > "$OUT/perfile.$1.tsv"
  while read -r f; do
    k=$((k + 1))
    rm -rf "$ONE"; mkdir -p "$ONE"
    (cd "$SRC" && cp --parents "$f" "$ONE")
    ir=$(count_ir "$ONE" "$d/$k.out" "$d/$k.txt" "$d/$k.log" 1)
    printf '%s\t%s\t%s\t%s\n' "$k" "$ir" "$(wc -c < "$SRC/$f")" "$f" >> "$OUT/perfile.$1.tsv"
  done < "$OUT/sample.txt"
}

sim=()
[ "$SIM" = 1 ] && sim=(--cache-sim=yes --branch-sim=yes)
irs=()
for ((i = 1; i <= RUNS; i++)); do
  cg=$OUT/callgrind.$i.out
  ir=$(count_ir "$ROOT" "$cg" "$OUT/bench.$i.txt" "$OUT/callgrind.$i.log" "$n" ${sim[@]+"${sim[@]}"})
  irs+=("$ir")
  printf 'run %d: Ir %s\n' "$i" "$ir"
  [ "$SIM" = 1 ] && sim_report "$cg"
  if [ "$PERFILE" = 1 ]; then
    per_file_run "$i"
    awk -F'\t' -v i="$i" '$1 > 0 { s += $2 } $1 == 0 { e = $2 } END {
      printf "run %d: per-file sum %d over %d processes, empty sample %d\n", i, s, NR - 1, e }' "$OUT/perfile.$i.tsv"
  fi
done
min=$(printf '%s\n' "${irs[@]}" | sort -n | head -1)
max=$(printf '%s\n' "${irs[@]}" | sort -n | tail -1)
printf 'Ir min %s  max %s  (%d runs; profiles in %s)\n' "$min" "$max" "$RUNS" "$OUT"
if [ "$PERFILE" = 1 ]; then
  # Each file's minimum over the runs (a run reads about 1.8K high now and then; README).
  cat "$OUT"/perfile.*.tsv | awk -F'\t' -v whole="$min" -v runs="$RUNS" '
    !($1 in m) || $2 < m[$1] { m[$1] = $2; b[$1] = $3; p[$1] = $4 }
    END {
      n = 0; for (k in m) if (k + 0 > n) n = k + 0
      e = m[0]
      printf "per-file (minimum of %d run%s per file; Ir, Ir net of the empty sample, bytes, file):\n", runs, (runs > 1 ? "s" : "")
      for (k = 1; k <= n; k++) {
        printf "  %3d %12d %12d %8d  %s\n", k, m[k], m[k] - e, b[k], p[k]
        s += m[k]
      }
      printf "per-file sum %d over %d processes; empty sample %d; sum net of it %d\n", s, n, e, s - n * e
      printf "whole sample %d (min); net of the empty sample %d\n", whole, whole - e
      printf "cross-file term (whole net - per-file net) %d\n", (whole - e) - (s - n * e)
    }'
fi
if [ "$ANNOTATE" != 0 ]; then
  callgrind_annotate --threshold=100 "$OUT/callgrind.1.out" 2>/dev/null | sed -n '1,/^--.*$/!p' | head -n "$((ANNOTATE + 25))"
fi

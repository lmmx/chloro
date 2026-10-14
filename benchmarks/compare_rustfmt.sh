#!/usr/bin/env bash
# Process-level comparison of chloro and rustfmt on named workloads, with hyperfine.
#
#   benchmarks/compare_rustfmt.sh --list
#   benchmarks/compare_rustfmt.sh [options] [WORKLOAD...]      # default: every workload
#
# Each workload is copied into a scratch directory (--work) so that both tools read the
# same bytes from the same place. See benchmarks/README.md for the method.
set -euo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=$(cd "$HERE/.." && pwd)
FIXTURES=$REPO/chloro-core/tests/conformance/fixtures/rust-analyzer

# ---------------------------------------------------------------------------------------
# Workloads. Columns: name, kind, source, crate root (crate kind only; "-" otherwise).
#   kind crate  : rustfmt on the crate root (it follows `mod` declarations); chloro on
#                 exactly the files rustfmt formatted. Variants: mc (chloro on all CPUs of
#                 --cpus) and 1c (both tools pinned to --core).
#   kind files  : many independent files, each a root without out-of-line modules; rustfmt
#                 gets every file as an argument, chloro the directory. Variants mc and 1c.
#   kind stdin  : one file on stdin to rustfmt and to examples/fmt_stdin, output to stdout.
#                 Pinned to --core.
#   kind inproc : in-process timers (examples/bench, examples/conform -i). Pinned to --core.
# Sources: fixtures:<path under fixtures/rust-analyzer>, registry:<crate dir under the
# cargo registry>, gen:one (a generated one-line file), bench, conform.
# ---------------------------------------------------------------------------------------
WORKLOADS='
fx-hir            crate   fixtures:hir                                   src/lib.rs
fx-hir-ty         crate   fixtures:hir-ty                                src/lib.rs
fx-ide-db         crate   fixtures:ide-db                                src/lib.rs
fx-syntax         crate   fixtures:syntax                                src/lib.rs
fx-parser         crate   fixtures:parser                                src/lib.rs
reg-tokio         crate   registry:tokio-1.53.3                          src/lib.rs
reg-regex-syntax  crate   registry:regex-syntax-0.8.11                   src/lib.rs
reg-clap-builder  crate   registry:clap_builder-4.6.7                    src/lib.rs
reg-winnow        crate   registry:winnow-0.7.15                         src/lib.rs
stdin-lints       stdin   fixtures:ide-db/src/generated/lints.rs         -
stdin-nodes       stdin   fixtures:syntax/src/ast/generated/nodes.rs     -
stdin-hir-lib     stdin   fixtures:hir/src/lib.rs                        -
stdin-hover-tests stdin   fixtures:ide/src/hover/tests.rs                -
stdin-config      stdin   fixtures:rust-analyzer/src/config.rs           -
tiny-test-data    files   fixtures:parser/test_data                      -
tiny-one          files   gen:one                                        -
inproc-bench      inproc  bench                                          -
inproc-conform    inproc  conform                                        -
'

usage() {
  cat <<USAGE
Usage: $0 [options] [WORKLOAD...]

Options (defaults in brackets):
  --list               print the workload table and exit
  --groups LIST        comma-separated groups to run [a,b,c,d,e]
                         a: crate kind, mc variant     b: crate kind, 1c variant
                         c: stdin kind                 d: files kind, mc and 1c variants
                         e: inproc kind
  -r, --runs N         hyperfine runs per command per round [10]
  -w, --warmup N       hyperfine warmup runs per command per round [2]
  --rounds K           repeat each hyperfine invocation K times, alternating command
                       order, to interleave the tools [1]
  --core N             CPU for pinned runs (1c variants, stdin, inproc) [3]
  --cpus LIST          taskset CPU list for mc variants; empty = unpinned [empty]
  --mode check|stdout  crate/files output: --check for both, or formatted text to
                       stdout for both (rustfmt --emit stdout, chloro print mode) [check]
  --input formatted|raw  workload bytes first formatted by rustfmt, so --check finds
                       nothing to report, or the bytes as found [formatted]
  --out DIR            results directory [benchmarks/results/<UTC timestamp>]
  --work DIR           scratch directory for workload copies [\${TMPDIR:-/tmp}/chloro-bench]
  --chloro BIN         chloro CLI [target/release/chloro]
  --fmt-stdin BIN      [target/release/examples/fmt_stdin]
  --bench BIN          [target/release/examples/bench]
  --conform BIN        [target/release/examples/conform]
  --baseline DIR       a second build's target/release directory: its chloro, fmt_stdin,
                       bench and conform run as "chloro-base" next to "chloro"
  --rustfmt BIN        [\`rustup which rustfmt\`, else rustfmt from PATH]
  --registry DIR       cargo registry source dir [~/.cargo/registry/src/index.crates.io-*]
  --fresh              rebuild the workload copies
USAGE
}

die() { echo "compare_rustfmt.sh: error: $*" >&2; exit 2; }
note() { echo "== $*" >&2; }

# --- options -----------------------------------------------------------------------------
GROUPS_SEL=a,b,c,d,e RUNS=10 WARMUP=2 ROUNDS=1 CORE=3 CPUS= MODE=check INPUT=formatted
OUT= WORK=${TMPDIR:-/tmp}/chloro-bench FRESH=0 LIST=0 BASELINE=
CHLORO=$REPO/target/release/chloro
FMT_STDIN=$REPO/target/release/examples/fmt_stdin
BENCH=$REPO/target/release/examples/bench
CONFORM=$REPO/target/release/examples/conform
RUSTFMT=
REGISTRY=
SELECTED=()
while [ $# -gt 0 ]; do
  case $1 in
    --list) LIST=1 ;;
    --groups) GROUPS_SEL=$2; shift ;;
    -r|--runs) RUNS=$2; shift ;;
    -w|--warmup) WARMUP=$2; shift ;;
    --rounds) ROUNDS=$2; shift ;;
    --core) CORE=$2; shift ;;
    --cpus) CPUS=$2; shift ;;
    --mode) MODE=$2; shift ;;
    --input) INPUT=$2; shift ;;
    --out) OUT=$2; shift ;;
    --work) WORK=$2; shift ;;
    --chloro) CHLORO=$2; shift ;;
    --fmt-stdin) FMT_STDIN=$2; shift ;;
    --bench) BENCH=$2; shift ;;
    --conform) CONFORM=$2; shift ;;
    --baseline) BASELINE=$2; shift ;;
    --rustfmt) RUSTFMT=$2; shift ;;
    --registry) REGISTRY=$2; shift ;;
    --fresh) FRESH=1 ;;
    -h|--help) usage; exit 0 ;;
    -*) usage >&2; die "unknown option $1" ;;
    *) SELECTED+=("$1") ;;
  esac
  shift
done

if [ "$LIST" = 1 ]; then
  echo "$WORKLOADS" | sed '/^$/d'
  exit 0
fi

case $MODE in check|stdout) ;; *) die "--mode must be check or stdout" ;; esac
case $INPUT in raw|formatted) ;; *) die "--input must be raw or formatted" ;; esac

# --- tools ---------------------------------------------------------------------------------
for t in hyperfine taskset jq; do
  command -v "$t" >/dev/null || die "required tool '$t' not found on PATH"
done
# The toolchain's rustfmt itself, not the rustup proxy in ~/.cargo/bin, which adds an exec
# and toolchain resolution to every run.
if [ -z "$RUSTFMT" ]; then
  if command -v rustup >/dev/null; then RUSTFMT=$(rustup which rustfmt 2>/dev/null || true); fi
  [ -n "$RUSTFMT" ] || RUSTFMT=$(command -v rustfmt || true)
fi
[ -n "$RUSTFMT" ] && [ -x "$RUSTFMT" ] || die "rustfmt not found (use --rustfmt)"
"$RUSTFMT" --version | grep -q '^rustfmt 1\.9\.' \
  || echo "warning: $("$RUSTFMT" --version) is not rustfmt 1.9.x, which chloro reproduces" >&2
for b in "$CHLORO" "$FMT_STDIN" "$BENCH" "$CONFORM"; do
  [ -x "$b" ] || die "binary $b not found; build with: cargo build --release -p chloro --features cli && cargo build --release -p chloro-core --examples"
done
if [ -n "$BASELINE" ]; then
  for b in chloro examples/fmt_stdin examples/bench examples/conform; do
    [ -x "$BASELINE/$b" ] || die "baseline binary $BASELINE/$b not found"
  done
fi
if [ -z "$REGISTRY" ]; then
  REGISTRY=$(ls -d "$HOME"/.cargo/registry/src/index.crates.io-* 2>/dev/null | head -1 || true)
fi
CPU_COUNT=$(nproc)
[ "$CORE" -lt "$CPU_COUNT" ] || die "--core $CORE is not below nproc=$CPU_COUNT"

mkdir -p "$WORK"
WORK=$(cd "$WORK" && pwd)
case $WORK in *[[:space:]]*) die "--work path must not contain whitespace (hyperfine -N splits commands on it)" ;; esac
[ -n "$OUT" ] || OUT=$HERE/results/$(date -u +%Y%m%dT%H%M%SZ)
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)

# rustfmt reads this empty config instead of searching parent directories for one, so a
# rustfmt.toml in a registry crate or above --work cannot change its options.
EMPTY_CFG=$WORK/empty-rustfmt.toml
: > "$EMPTY_CFG"
RF=("$RUSTFMT" --edition 2024 --config-path "$EMPTY_CFG")

ingroup() { case ",$GROUPS_SEL," in *",$1,"*) return 0 ;; *) return 1 ;; esac; }
selected() {
  [ ${#SELECTED[@]} -eq 0 ] && return 0
  local s; for s in "${SELECTED[@]}"; do [ "$s" = "$1" ] && return 0; done; return 1
}
resolve_source() { # source spec -> path
  case $1 in
    fixtures:*) echo "$FIXTURES/${1#fixtures:}" ;;
    registry:*) [ -n "$REGISTRY" ] || die "no cargo registry found (use --registry)"
                echo "$REGISTRY/${1#registry:}" ;;
    *) die "unknown source $1" ;;
  esac
}
# Paths rustfmt formats, from its --verbose "Formatting <path>" lines (one per file, also
# for files that need no change; `-l` lists only changed files).
rustfmt_touched() { "${RF[@]}" --check -v --color never "$@" 2>/dev/null | sed -n 's/^Formatting //p' | sort -u || true; }

# --- workload preparation ------------------------------------------------------------------
# Creates $W/tree (the bytes both tools read), $W/files.txt (relative paths), $W/meta.
prepare() {
  local name=$1 kind=$2 source=$3 root=$4
  local W=$WORK/$INPUT/$name
  if [ "$FRESH" = 0 ] && [ -f "$W/meta" ]; then echo "$W"; return; fi
  rm -rf "$W"; mkdir -p "$W/tree"
  case $kind in
    crate)
      local src; src=$(resolve_source "$source")
      [ -f "$src/$root" ] || die "$name: crate root $src/$root not found"
      rustfmt_touched "$src/$root" > "$W/touched.abs"
      [ -s "$W/touched.abs" ] || die "$name: rustfmt formatted no file from $src/$root (parse error? run rustfmt by hand)"
      sed "s|^$src/||" "$W/touched.abs" > "$W/files.txt"
      grep -q '^/' "$W/files.txt" && die "$name: rustfmt formatted files outside $src"
      (cd "$src" && xargs -a "$W/files.txt" cp --parents -t "$W/tree")
      echo "$root" > "$W/root"
      ;;
    files)
      if [ "$source" = gen:one ]; then
        printf 'fn main() {}\n' > "$W/tree/one.rs"
        echo one.rs > "$W/files.txt"
      else
        local src; src=$(resolve_source "$source")
        [ -d "$src" ] || die "$name: $src not found"
        # Keep files rustfmt parses on their own (drops the parser's error-recovery inputs).
        (cd "$src" && find . -name '*.rs' | sed 's|^\./||' | sort) > "$W/candidates.txt"
        : > "$W/files.txt"
        while read -r f; do
          if "${RF[@]}" --emit stdout "$src/$f" >/dev/null 2>&1; then echo "$f" >> "$W/files.txt"; fi
        done < "$W/candidates.txt"
        (cd "$src" && xargs -a "$W/files.txt" cp --parents -t "$W/tree")
      fi
      ;;
    stdin)
      local src; src=$(resolve_source "$source")
      [ -f "$src" ] || die "$name: $src not found"
      cp "$src" "$W/tree/input.rs"
      echo input.rs > "$W/files.txt"
      ;;
    *) die "prepare: kind $kind" ;;
  esac
  if [ "$INPUT" = formatted ]; then
    if [ "$kind" = crate ]; then
      "${RF[@]}" "$W/tree/$root" 2>/dev/null || true
    else
      (cd "$W/tree" && xargs -a "$W/files.txt" "${RF[@]}" 2>/dev/null) || true
    fi
    if [ "$kind" = files ]; then
      # rustfmt's output is not always parseable by rustfmt (parser/test_data has two such
      # files: `|| .. .field` becomes `|| ...field`); drop files it can no longer parse.
      local f keep=()
      while read -r f; do
        if "${RF[@]}" --emit stdout "$W/tree/$f" >/dev/null 2>&1; then keep+=("$f"); else rm "$W/tree/$f"; echo "$f" >> "$W/dropped_after_format.txt"; fi
      done < "$W/files.txt"
      printf '%s\n' "${keep[@]}" > "$W/files.txt"
    fi
  fi
  # Verify: the tree holds exactly the files rustfmt formats from it.
  if [ "$kind" = crate ]; then
    rustfmt_touched "$W/tree/$root" | sed "s|^$W/tree/||" > "$W/check.txt"
  elif [ "$kind" = files ]; then
    (cd "$W/tree" && xargs -a "$W/files.txt" "${RF[@]}" --check -v --color never 2>/dev/null \
      | sed -n 's/^Formatting //p' | sed "s|^$W/tree/||; s|^\./||" | sort > "$W/check.txt") || true
    local lines; lines=$(wc -l < "$W/check.txt")
    [ "$lines" = "$(wc -l < "$W/files.txt")" ] || die "$name: rustfmt formatted $lines files, expected $(wc -l < "$W/files.txt") (module following?)"
  fi
  if [ "$kind" != stdin ]; then
    (cd "$W/tree" && find . -name '*.rs' | sed 's|^\./||' | sort) > "$W/present.txt"
    sort "$W/files.txt" | cmp -s - "$W/check.txt" || die "$name: rustfmt's file set differs from files.txt (see $W)"
    cmp -s "$W/check.txt" "$W/present.txt" || die "$name: tree holds files rustfmt does not format (see $W)"
  fi
  # Output agreement, recorded for the reader: files whose formatted text is identical.
  local agree=-
  if [ "$kind" = stdin ]; then
    "${RF[@]}" < "$W/tree/input.rs" > "$W/out.rustfmt" 2>/dev/null || true
    "$FMT_STDIN" < "$W/tree/input.rs" > "$W/out.chloro" 2>/dev/null || true
    if cmp -s "$W/out.rustfmt" "$W/out.chloro"; then agree=1/1; else agree=0/1; fi
  else
    rm -rf "$W/agree_r" "$W/agree_c"; cp -r "$W/tree" "$W/agree_r"; cp -r "$W/tree" "$W/agree_c"
    if [ "$kind" = crate ]; then
      "${RF[@]}" "$W/agree_r/$root" >/dev/null 2>&1 || true
    else
      (cd "$W/agree_r" && xargs -a "$W/files.txt" "${RF[@]}" >/dev/null 2>&1) || true
    fi
    "$CHLORO" --write "$W/agree_c" >/dev/null 2>&1 || true
    local n differ
    n=$(wc -l < "$W/files.txt")
    differ=$(diff -rq "$W/agree_r" "$W/agree_c" | wc -l || true)
    agree=$((n - differ))/$n
    rm -rf "$W/agree_r" "$W/agree_c"
  fi
  local bytes; bytes=$(cd "$W/tree" && xargs -a "$W/files.txt" cat | wc -c)
  {
    echo "name=$name"; echo "kind=$kind"; echo "source=$source"; echo "root=$root"
    echo "input=$INPUT"; echo "files=$(wc -l < "$W/files.txt")"; echo "bytes=$bytes"
    echo "identical_outputs=$agree"
  } > "$W/meta"
  echo "$W"
}

# --- running -------------------------------------------------------------------------------
HF_METRICS=time_wall_clock,time_user,time_system,memory_peak_resident

# validate LOG CMD...: run once, require exit 0/1 and no error/panic on stderr.
validate() {
  local log=$1; shift
  local rc=0
  "$@" > /dev/null 2> "$log.stderr" < "${VALIDATE_STDIN:-/dev/null}" || rc=$?
  echo "exit=$rc cmd=$*" >> "$log"
  if [ "$rc" -gt 1 ] || grep -Eq '^error|panicked|^Errors:' "$log.stderr"; then
    cat "$log.stderr" >&2
    die "validation failed (exit $rc): $*"
  fi
}

# run_hf ID PIN STDIN LABEL CMD [LABEL CMD]...: K rounds of hyperfine, order alternating.
run_hf() {
  local id=$1 pin=$2 stdin=$3; shift 3
  local -a labels=() cmds=()
  while [ $# -gt 0 ]; do labels+=("$1"); cmds+=("$2"); shift 2; done
  local -a pinpre=()
  [ -n "$pin" ] && pinpre=(taskset -c "$pin")
  local -a inopt=()
  [ -n "$stdin" ] && inopt=(--input "$stdin")
  local i r n=${#labels[@]}
  : > "$OUT/$id.validate.log"
  for ((i = 0; i < n; i++)); do
    # shellcheck disable=SC2086
    VALIDATE_STDIN=$stdin validate "$OUT/$id.validate.log" "${pinpre[@]}" ${cmds[$i]}
  done
  for ((r = 1; r <= ROUNDS; r++)); do
    local -a args=()
    for ((i = 0; i < n; i++)); do
      local j=$i
      [ $((r % 2)) = 0 ] && j=$((n - 1 - i))
      args+=(-n "${labels[$j]}" "${cmds[$j]}")
    done
    printf '%q ' "${pinpre[@]}" hyperfine -N -i -w "$WARMUP" -r "$RUNS" --metrics "$HF_METRICS" \
      "${inopt[@]}" --export-json "$OUT/$id.r$r.json" "${args[@]}" >> "$OUT/commands.txt"
    echo >> "$OUT/commands.txt"
    note "$id round $r/$ROUNDS${pin:+ (taskset -c $pin)}"
    "${pinpre[@]}" hyperfine -N -i -w "$WARMUP" -r "$RUNS" --metrics "$HF_METRICS" \
      "${inopt[@]}" --export-json "$OUT/$id.r$r.json" "${args[@]}" >&2
  done
}

chloro_bins() { # role -> "label path" lines for chloro and the optional baseline
  echo "chloro $1"
  [ -n "$BASELINE" ] && echo "chloro-base $2"
  return 0
}

run_crate_or_files() {
  local name=$1 kind=$2 W=$3 variant=$4 pin=$5
  local rcmd ccmd_args
  if [ "$kind" = crate ]; then
    local root; root=$(cat "$W/root")
    if [ "$MODE" = check ]; then rcmd="${RF[*]} --check $W/tree/$root"; else rcmd="${RF[*]} --emit stdout $W/tree/$root"; fi
  else
    local list; list=$(sed "s|^|$W/tree/|" "$W/files.txt" | tr '\n' ' ')
    if [ "$MODE" = check ]; then rcmd="${RF[*]} --check $list"; else rcmd="${RF[*]} --emit stdout $list"; fi
  fi
  if [ "$MODE" = check ]; then ccmd_args="--check $W/tree"; else ccmd_args="$W/tree"; fi
  local -a pairs=(rustfmt "$rcmd")
  while read -r label bin; do pairs+=("$label" "$bin $ccmd_args"); done \
    < <(chloro_bins "$CHLORO" "$BASELINE/chloro")
  cp "$W/meta" "$OUT/$name.$variant.meta"
  { echo "variant=$variant"; echo "mode=$MODE"; echo "pin=${pin:-none}"; } >> "$OUT/$name.$variant.meta"
  run_hf "$name.$variant" "$pin" "" "${pairs[@]}"
}

run_stdin() {
  local name=$1 W=$2
  local -a pairs=(rustfmt "${RF[*]}")
  while read -r label bin; do pairs+=("$label" "$bin"); done \
    < <(chloro_bins "$FMT_STDIN" "$BASELINE/examples/fmt_stdin")
  cp "$W/meta" "$OUT/$name.1c.meta"
  { echo "variant=1c"; echo "mode=stdout"; echo "pin=$CORE"; } >> "$OUT/$name.1c.meta"
  run_hf "$name.1c" "$CORE" "$W/tree/input.rs" "${pairs[@]}"
}

run_inproc() {
  local name=$1 what=$2
  case $what in
    bench)
      # examples/bench times format_source in process (best of -n 5); one line per run.
      local r i
      : > "$OUT/$name.1c.bench.txt"
      for ((r = 1; r <= ROUNDS; r++)); do
        while read -r label bin; do
          note "$name $label round $r/$ROUNDS (taskset -c $CORE)"
          echo "taskset -c $CORE $bin -n 5" >> "$OUT/commands.txt"
          printf '%s\t%s\n' "$label" "$(taskset -c "$CORE" "$bin" -n 5 | head -1)" >> "$OUT/$name.1c.bench.txt"
        done < <(if [ $((r % 2)) = 1 ]; then chloro_bins "$BENCH" "$BASELINE/examples/bench"; else chloro_bins "$BENCH" "$BASELINE/examples/bench" | tac; fi)
      done
      cat "$OUT/$name.1c.bench.txt" >&2
      ;;
    conform)
      local -a pairs=()
      while read -r label bin; do pairs+=("$label" "$bin -i"); done \
        < <(chloro_bins "$CONFORM" "$BASELINE/examples/conform")
      { echo "name=$name"; echo "kind=inproc"; echo "variant=1c"; echo "pin=$CORE"; } > "$OUT/$name.1c.meta"
      local saved_runs=$RUNS saved_warm=$WARMUP
      RUNS=$(( RUNS < 3 ? RUNS : 3 )); WARMUP=$(( WARMUP < 1 ? WARMUP : 1 ))
      run_hf "$name.1c" "$CORE" "" "${pairs[@]}"
      RUNS=$saved_runs WARMUP=$saved_warm
      ;;
  esac
}

# --- environment record --------------------------------------------------------------------
{
  echo "date_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "commit=$(git -C "$REPO" rev-parse HEAD 2>/dev/null || echo unknown)"
  echo "worktree_dirty=$(git -C "$REPO" status --porcelain --untracked-files=no 2>/dev/null | wc -l)"
  echo "host_cpu=$(sed -n 's/^model name[[:space:]]*: //p' /proc/cpuinfo | head -1)"
  echo "nproc=$CPU_COUNT"
  echo "kernel=$(uname -r)"
  echo "loadavg_start=$(cut -d' ' -f1-3 /proc/loadavg)"
  echo "cpu${CORE}_stat_start=$(grep "^cpu$CORE " /proc/stat | cut -d' ' -f2-)"
  echo "rustfmt=$("$RUSTFMT" --version) path=$RUSTFMT"
  echo "rustc=$(rustc --version 2>/dev/null || echo unknown)"
  echo "hyperfine=$(hyperfine --version)"
  echo "chloro=$CHLORO sha256=$(sha256sum "$CHLORO" | cut -c1-16)"
  echo "fmt_stdin=$FMT_STDIN sha256=$(sha256sum "$FMT_STDIN" | cut -c1-16)"
  echo "bench=$BENCH sha256=$(sha256sum "$BENCH" | cut -c1-16)"
  echo "baseline=${BASELINE:-none}"
  echo "runs=$RUNS warmup=$WARMUP rounds=$ROUNDS core=$CORE cpus=${CPUS:-unpinned} mode=$MODE input=$INPUT groups=$GROUPS_SEL"
  echo "work=$WORK"
} > "$OUT/env.txt"
: > "$OUT/commands.txt"

# --- main loop -----------------------------------------------------------------------------
while read -r name kind source root; do
  [ -z "$name" ] && continue
  selected "$name" || continue
  case $kind in
    crate)
      ingroup a || ingroup b || continue
      W=$(prepare "$name" "$kind" "$source" "$root")
      note "$name: $(tr '\n' ' ' < "$W/meta")"
      ingroup a && run_crate_or_files "$name" crate "$W" mc "$CPUS"
      ingroup b && run_crate_or_files "$name" crate "$W" 1c "$CORE"
      ;;
    files)
      ingroup d || continue
      W=$(prepare "$name" "$kind" "$source" "$root")
      note "$name: $(tr '\n' ' ' < "$W/meta")"
      run_crate_or_files "$name" files "$W" mc "$CPUS"
      run_crate_or_files "$name" files "$W" 1c "$CORE"
      ;;
    stdin)
      ingroup c || continue
      W=$(prepare "$name" "$kind" "$source" "$root")
      note "$name: $(tr '\n' ' ' < "$W/meta")"
      run_stdin "$name" "$W"
      ;;
    inproc)
      ingroup e || continue
      run_inproc "$name" "$source"
      ;;
  esac
done <<< "$WORKLOADS"

# /proc/stat fields: user nice system idle iowait irq softirq steal ... (USER_HZ ticks).
# Steal is time the hypervisor ran something else while this vCPU wanted to run.
{
  echo "loadavg_end=$(cut -d' ' -f1-3 /proc/loadavg)"
  end=$(grep "^cpu$CORE " /proc/stat | cut -d' ' -f2-)
  echo "cpu${CORE}_stat_end=$end"
  start=$(sed -n "s/^cpu${CORE}_stat_start=//p" "$OUT/env.txt")
  read -r -a s0 <<< "$start"; read -r -a s1 <<< "$end"
  echo "cpu${CORE}_steal_ticks=$(( s1[7] - s0[7] )) cpu${CORE}_busy_ticks=$(( s1[0] + s1[2] - s0[0] - s0[2] )) cpu${CORE}_idle_ticks=$(( s1[3] - s0[3] ))"
} >> "$OUT/env.txt"
note "results in $OUT"
if command -v python3 >/dev/null; then
  python3 "$HERE/report.py" "$OUT"
fi

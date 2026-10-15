#!/bin/bash
# EXP-023: CLI output identity: BASE and NEW on copies of the workloads, at 1, 2 and 4 threads.
# check (formatted and raw trees), write (raw trees; the written trees compared), print
# (stdout compared sorted, and exactly at 1 thread), check with a cold cache at 4 threads.
# usage: identity.sh BASE NEW WORKDIR   (prints each differing comparison and a count)
BASE=$1; NEW=$2; W=$3
mkdir -p $W
fail=0; n=0
run() { # label tree args...
  local label=$1 tree=$2; shift 2
  "$BASE" "$@" "$tree" > $W/b.out 2> $W/b.err; echo $? > $W/b.code
  "$NEW" "$@" "$tree" > $W/n.out 2> $W/n.err; echo $? > $W/n.code
}
cmp_exact() {
  n=$((n+1))
  if cmp -s $W/b.out $W/n.out && cmp -s $W/b.err $W/n.err && cmp -s $W/b.code $W/n.code; then :; else echo "DIFF $1"; fail=$((fail+1)); fi
}
cmp_sorted() {
  n=$((n+1))
  if cmp -s <(sort $W/b.out) <(sort $W/n.out) && cmp -s $W/b.err $W/n.err && cmp -s $W/b.code $W/n.code; then :; else echo "DIFF $1"; fail=$((fail+1)); fi
}
WL="fx-hir fx-hir-ty fx-ide-db fx-syntax fx-parser reg-tokio reg-regex-syntax reg-clap-builder reg-winnow"
declare -A SRC
for c in $WL; do SRC[$c-fmt]=/tmp/chloro-bench/formatted/$c/tree; SRC[$c-raw]=/tmp/chloro-bench/raw/$c/tree; done
SRC[fixtures]=/home/user/chloro/chloro-core/tests/conformance/fixtures
for key in "${!SRC[@]}"; do
  src=${SRC[$key]}
  rm -rf $W/tree; cp -r $src $W/tree
  for th in 1 2 4; do
    run "$key check t$th" $W/tree --check --no-cache --threads $th; cmp_exact "$key check t$th"
    run "$key print t$th" $W/tree --no-cache --threads $th
    if [ $th = 1 ]; then cmp_exact "$key print t$th"; else cmp_sorted "$key print t$th"; fi
    if [[ $key != *-fmt ]]; then
      rm -rf $W/wb $W/wn; cp -r $src $W/wb; cp -r $src $W/wn
      "$BASE" --write --no-cache --threads $th $W/wb > $W/b.out 2> $W/b.err; echo $? > $W/b.code
      "$NEW" --write --no-cache --threads $th $W/wn > $W/n.out 2> $W/n.err; echo $? > $W/n.code
      cmp_exact "$key write t$th (streams)"
      n=$((n+1)); if ! diff -rq $W/wb $W/wn > /dev/null; then echo "DIFF $key write t$th (trees)"; fail=$((fail+1)); fi
    fi
  done
  rm -rf $W/cb $W/cn; mkdir -p $W/cb $W/cn
  "$BASE" --check --cache-dir $W/cb --threads 4 $W/tree > $W/b.out 2> $W/b.err; echo $? > $W/b.code
  "$NEW" --check --cache-dir $W/cn --threads 4 $W/tree > $W/n.out 2> $W/n.err; echo $? > $W/n.code
  cmp_exact "$key check cold-cache t4"
  printf '%s: exit %s\n' "$key" "$(cat $W/n.code)"
done
rm -rf $W/tree $W/wb $W/wn $W/cb $W/cn
echo "comparisons: $n, differing: $fail"

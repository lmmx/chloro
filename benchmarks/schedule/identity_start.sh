#!/bin/bash
# EXP-029: the written trees of START (a CLI older than --threads and --no-cache, such as
# 884f9c9's, which rejects identity.sh's options) against NEW --write --no-cache --threads N
# (N = 1, 2, 4), on the nine formatted and raw crates and the fixtures; prints every differing
# file once per (tree, N) and the union.
# usage: identity_start.sh START NEW WORKDIR
START=$1; NEW=$2; W=$3
mkdir -p $W; : > $W/diffs
WL="fx-hir fx-hir-ty fx-ide-db fx-syntax fx-parser reg-tokio reg-regex-syntax reg-clap-builder reg-winnow"
declare -A SRC
for c in $WL; do SRC[$c-fmt]=/tmp/chloro-bench/formatted/$c/tree; SRC[$c-raw]=/tmp/chloro-bench/raw/$c/tree; done
SRC[fixtures]=/home/user/chloro/chloro-core/tests/conformance/fixtures
n=0
for key in "${!SRC[@]}"; do
  src=${SRC[$key]}
  rm -rf $W/s; cp -r $src $W/s; "$START" --write $W/s > /dev/null 2>&1; echo "$key start exit $?"
  for th in 1 2 4; do
    rm -rf $W/n; cp -r $src $W/n; "$NEW" --write --no-cache --threads $th $W/n > /dev/null 2>&1
    n=$((n+1))
    diff -rq $W/s $W/n | sed "s|^|$key t$th: |" | tee -a $W/diffs
  done
done
rm -rf $W/s $W/n
echo "tree comparisons: $n; differing lines: $(wc -l < $W/diffs)"
echo "distinct files:"; sed -E 's/^[^:]*: Files [^ ]*\/s\/([^ ]*) and .*/\1/' $W/diffs | sort -u

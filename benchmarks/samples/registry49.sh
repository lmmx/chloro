#!/usr/bin/env bash
# Writes the registry validation sample of benchmarks/profile.sh (EXP-027) to stdout:
#
#   benchmarks/samples/registry49.sh > benchmarks/samples/registry49.txt
#
# The rule: the `.rs` files (`find -type f`) of the 577 crates of the registry corpus
# (docs/journal/2026-10-14-autoresearch/corpus/registry-crates.txt) under
# ${CARGO_HOME:-~/.cargo}/registry/src, paths relative to it sorted bytewise (LC_ALL=C);
# with T files, N = T / 49 (integer division) and candidate k (k = 1..49) is line k*N. A
# candidate that fails a check is replaced by the next line that passes. A file passes when
# `rustfmt --edition 2024` (stdin, empty configuration) exits 0, rustfmt's output is a fixed
# point of rustfmt, and chloro's output (examples/fmt_stdin) is byte-identical to rustfmt's.
# The rejects are listed in the header with their reason.
#
# Needs rustfmt 1.9.0 on PATH and target/release/examples/fmt_stdin.
set -euo pipefail

HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
CRATES=$REPO/docs/journal/2026-10-14-autoresearch/corpus/registry-crates.txt
REG=${CARGO_HOME:-$HOME/.cargo}/registry/src
STDIN=$REPO/target/release/examples/fmt_stdin
K=49

die() { echo "registry49.sh: error: $*" >&2; exit 2; }
command -v rustfmt >/dev/null || die "rustfmt not found"
[ -x "$STDIN" ] || die "$STDIN not found; build with: cargo build --release -p chloro-core --examples"
[ -f "$CRATES" ] || die "$CRATES not found"
[ -d "$REG" ] || die "$REG not found"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
: > "$TMP/empty.toml"

# The registry's index directory that holds every crate of the corpus.
index=
for d in "$REG"/index.crates.io-*/; do
  d=$(basename "$d")
  missing=$(while read -r c; do [ -d "$REG/$d/$c" ] || echo "$c"; done < "$CRATES" | wc -l)
  [ "$missing" = 0 ] && { index=$d; break; }
done
[ -n "$index" ] || die "no index directory under $REG holds every crate of $CRATES"

(cd "$REG" && while read -r c; do find "$index/$c" -type f -name '*.rs'; done < "$CRATES") |
  LC_ALL=C sort > "$TMP/all.txt"
total=$(wc -l < "$TMP/all.txt")
N=$((total / K))

# check PATH: prints nothing when the file passes, the reason otherwise.
check() {
  local f=$REG/$1 rc=0
  rustfmt --edition 2024 --config-path "$TMP/empty.toml" < "$f" > "$TMP/a.rs" 2> /dev/null || rc=$?
  if [ "$rc" != 0 ]; then echo "rustfmt exits $rc"; return; fi
  rustfmt --edition 2024 --config-path "$TMP/empty.toml" < "$TMP/a.rs" > "$TMP/b.rs" 2> /dev/null ||
    { echo "rustfmt rejects its own output"; return; }
  cmp -s "$TMP/a.rs" "$TMP/b.rs" || { echo "rustfmt's output is not a fixed point"; return; }
  "$STDIN" < "$f" > "$TMP/c.rs" 2> /dev/null || { echo "chloro fails"; return; }
  cmp -s "$TMP/a.rs" "$TMP/c.rs" || echo "chloro's output differs from rustfmt's"
}

: > "$TMP/picked.txt"; : > "$TMP/rejected.txt"
next=1
for ((k = 1; k <= K; k++)); do
  line=$((k * N))
  [ "$line" -ge "$next" ] || line=$next
  while :; do
    [ "$line" -le "$total" ] || die "ran out of files after candidate $k"
    f=$(sed -n "${line}p" "$TMP/all.txt")
    why=$(check "$f")
    [ -z "$why" ] && break
    printf '#   %d %s: %s\n' "$line" "$f" "$why" >> "$TMP/rejected.txt"
    line=$((line + 1))
  done
  echo "$f" >> "$TMP/picked.txt"
  next=$((line + 1))
done
bytes=$(cd "$REG" && xargs -a "$TMP/picked.txt" cat | wc -c)

cat <<HEAD
# The registry validation sample of benchmarks/profile.sh (EXP-027): 49 files, $bytes bytes.
# root: cargo-registry
# Paths are relative to \${CARGO_HOME:-~/.cargo}/registry/src. Written by
# benchmarks/samples/registry49.sh with $(rustfmt --version).
# Rule: the .rs files (find -type f) of the 577 crates of
# docs/journal/2026-10-14-autoresearch/corpus/registry-crates.txt under
# $index, sorted bytewise: $total files; N = $total / 49 = $N;
# candidate k is line k*N (k = 1..49), and a candidate that fails a check is replaced by the
# next line that passes. A file passes when rustfmt --edition 2024 (stdin, empty
# configuration) exits 0, its output is a fixed point of rustfmt, and chloro's output
# (examples/fmt_stdin) is byte-identical to rustfmt's.
# Rejected ($(wc -l < "$TMP/rejected.txt"); line, path, reason):
HEAD
cat "$TMP/rejected.txt"
cat "$TMP/picked.txt"

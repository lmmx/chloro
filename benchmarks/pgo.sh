#!/usr/bin/env bash
# Profile-guided build of the chloro CLI.
#
#   benchmarks/pgo.sh [TARGET_ROOT] [FEATURES]
#
# Builds an instrumented CLI (`-Cprofile-generate`), trains it on the conformance fixtures
# through the CLI's `--check` and through `bench -n 1`, merges the profiles with the
# toolchain's `llvm-profdata`, and builds the optimised CLI with `-Cprofile-use`. The
# result is `$TARGET_ROOT/pgo/release/chloro` (default `target/pgo/release/chloro`); the
# instrumented build and the profiles stay under `$TARGET_ROOT/pgo-gen/`. Needs the
# `llvm-tools-preview` rustup component (`rustup component add llvm-tools-preview`).
set -euo pipefail

cd "$(dirname "$0")/.."
root=${1:-target}
features=${2:-cli}
gen="$root/pgo-gen"
profdir="$gen/profiles"
host=$(rustc -vV | sed -n 's/^host: //p')
llvm_bin="$(rustc --print sysroot)/lib/rustlib/$host/bin"
profdata="$llvm_bin/llvm-profdata"
if [ ! -x "$profdata" ]; then
    echo "llvm-profdata not found at $profdata; run: rustup component add llvm-tools-preview" >&2
    exit 1
fi
fixtures=chloro-core/tests/conformance/fixtures

rm -rf "$profdir"
mkdir -p "$profdir"

echo "== instrumented build"
RUSTFLAGS="-Cprofile-generate=$profdir ${RUSTFLAGS:-}" \
    cargo build --release -p chloro --features "$features" --target-dir "$gen"
RUSTFLAGS="-Cprofile-generate=$profdir ${RUSTFLAGS:-}" \
    cargo build --release -p chloro-core --example bench --target-dir "$gen"

echo "== training"
"$gen/release/chloro" --check "$fixtures" > /dev/null 2>&1 || true
"$gen/release/examples/bench" -n 1 > /dev/null

echo "== merging $(ls "$profdir"/*.profraw | wc -l) profiles"
"$profdata" merge -o "$profdir/merged.profdata" "$profdir"/*.profraw

echo "== optimised build"
RUSTFLAGS="-Cprofile-use=$profdir/merged.profdata -Cllvm-args=-pgo-warn-missing-function ${RUSTFLAGS:-}" \
    cargo build --release -p chloro --features "$features" --target-dir "$root/pgo"

echo "built $root/pgo/release/chloro"

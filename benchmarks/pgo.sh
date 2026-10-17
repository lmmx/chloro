#!/usr/bin/env bash
# Profile-guided build of the chloro CLI.
#
#   benchmarks/pgo.sh [--phase all|train|use] [--target TRIPLE] [TARGET_ROOT] [FEATURES]
#
# Builds an instrumented CLI (`-Cprofile-generate`), trains it on the conformance fixtures
# through the CLI's `--check` and through `bench -n 1`, merges the profiles with the
# toolchain's `llvm-profdata`, and builds the optimised CLI with `-Cprofile-use`. The
# result is `$TARGET_ROOT/pgo/release/chloro` (default `target/pgo/release/chloro`); the
# instrumented build and the profiles stay under `$TARGET_ROOT/pgo-gen/`. Needs the
# `llvm-tools-preview` rustup component (`rustup component add llvm-tools-preview`).
#
# --phase train   stop after the merge: the profile is `$TARGET_ROOT/pgo-gen/profiles/
#                 merged.profdata`. The release workflow runs this phase and lets its own
#                 build use the profile (.github/workflows/binaries.yml).
# --phase use     the `-Cprofile-use` builds only, from a profile a train phase left.
# --target T      pass `--target T` to every build (binaries then sit under `<dir>/T/
#                 release/`). Defaults to `$CARGO_BUILD_TARGET` when that is set, which
#                 cargo would apply anyway. Training runs the instrumented binaries, so T
#                 has to be the host triple. A profile only matches a build made the same
#                 way: symbol names differ with and without --target, so train with
#                 --target when the optimised build passes it (the release workflow does).
set -euo pipefail

phase=all
target=${CARGO_BUILD_TARGET:-}
while [ $# -gt 0 ]; do
    case $1 in
        --phase) phase=$2; shift 2 ;;
        --target) target=$2; shift 2 ;;
        --) shift; break ;;
        -*) echo "unknown option $1" >&2; exit 2 ;;
        *) break ;;
    esac
done
case $phase in all|train|use) ;; *) echo "--phase must be all, train or use" >&2; exit 2 ;; esac

cd "$(dirname "$0")/.."
root=${1:-target}
features=${2:-cli}
# rustc resolves -Cprofile-use relative to the directory it is invoked from, which cargo
# sets per crate, so the root has to be absolute.
case $root in /*) ;; *) root=$PWD/$root ;; esac
gen="$root/pgo-gen"
profdir="$gen/profiles"
profile="$profdir/merged.profdata"
host=$(rustc -vV | sed -n 's/^host: //p')
case ${target:-$host} in *-windows*) exe=.exe ;; *) exe= ;; esac
# rustc on Windows needs a Windows path in -Cprofile-generate/-use; under Git Bash the
# shell's paths are /c/... (cygpath -m gives C:/...).
native() { if command -v cygpath > /dev/null; then cygpath -m "$1"; else printf '%s\n' "$1"; fi; }
target_args=()
subdir=release
if [ -n "$target" ]; then
    target_args=(--target "$target")
    subdir=$target/release
fi
fixtures=chloro-core/tests/conformance/fixtures

if [ "$phase" != use ]; then
    if [ "${target:-$host}" != "$host" ]; then
        echo "cannot train for $target on $host: the instrumented binary has to run here" >&2
        exit 3
    fi
    llvm_bin="$(rustc --print sysroot)/lib/rustlib/$host/bin"
    profdata="$llvm_bin/llvm-profdata$exe"
    if [ ! -x "$profdata" ]; then
        echo "llvm-profdata not found at $profdata; run: rustup component add llvm-tools-preview" >&2
        exit 1
    fi

    rm -rf "$profdir"
    mkdir -p "$profdir"
    gen_flags="-Cprofile-generate=$(native "$profdir") ${RUSTFLAGS:-}"

    echo "== instrumented build"
    RUSTFLAGS="$gen_flags" cargo build --release ${target_args[@]+"${target_args[@]}"} -p chloro \
        --features "$features" --target-dir "$gen"
    RUSTFLAGS="$gen_flags" cargo build --release ${target_args[@]+"${target_args[@]}"} -p chloro-core \
        --example bench --target-dir "$gen"

    echo "== training"
    "$gen/$subdir/chloro$exe" --check "$fixtures" > /dev/null 2>&1 || true
    "$gen/$subdir/examples/bench$exe" -n 1 > /dev/null

    echo "== merging $(ls "$profdir"/*.profraw | wc -l) profiles"
    "$profdata" merge -o "$profile" "$profdir"/*.profraw
    echo "profile $profile ($(wc -c < "$profile") bytes)"
fi

if [ "$phase" != train ]; then
    if [ ! -s "$profile" ]; then
        echo "no profile at $profile; run the train phase first" >&2
        exit 1
    fi
    echo "== optimised build"
    # -pgo-warn-missing-function makes LLVM warn for every function the profile has no
    # record of (a profile from other code); the warnings do not fail a `-D warnings` build.
    use_flags="-Cprofile-use=$(native "$profile") -Cllvm-args=-pgo-warn-missing-function ${RUSTFLAGS:-}"
    RUSTFLAGS="$use_flags" cargo build --release ${target_args[@]+"${target_args[@]}"} -p chloro \
        --features "$features" --target-dir "$root/pgo"
    # The in-process benchmark with the same profile, so `bench -n 5` and `profile.sh` can
    # measure the profile-guided code too.
    RUSTFLAGS="$use_flags" cargo build --release ${target_args[@]+"${target_args[@]}"} -p chloro-core \
        --example bench --target-dir "$root/pgo"
    echo "built $root/pgo/$subdir/chloro$exe and $root/pgo/$subdir/examples/bench$exe"
fi
